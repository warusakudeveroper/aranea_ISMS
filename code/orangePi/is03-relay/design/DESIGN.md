# is03-relay 詳細設計（Orange Pi Zero3 / BLEローカル受信サーバ）

作成日: 2025-12-24  
対象: Armbian 25.5.1 (Orange Pi Zero3)  
目的: is01/is02などESP系デバイスからのBLE/HTTPデータをローカル受信・蓄積・可視化し、再送や冗長構成を支援する。旧仮実装からの問題を解消し、運用手順とモジュール役割を明確化する。

## 1. 現状課題（旧実装）
- BLE受信のみの簡易実装で、クラウド/ローカル再送や冗長構成を想定していない。
- 設定項目が分散・環境依存で、固定IPやログ場所の変更が面倒。
- UI/RESTのアクセス制御がなく、LAN外からも見えてしまうリスク。
- ストア/キューの上限やバックオフポリシーが無く、障害時に無制限蓄積する恐れ。

## 2. 目標像
- **ローカル受信＋可視化**: BLEスキャン/HTTP ingest→リングバッファ/SQLite保存→Web UIで閲覧（WSは廃止しHTTPポーリングのみ）。
- **再送準備**: Forwarderキューとバックオフ、冗長系（プライマリ/セカンダリURL）を持ち、将来的なクラウド/ローカル再送を容易にする。
- **運用性**: systemd自動起動、固定IP前提、設定のホットリロード、ログ閲覧API/UI。
- **安全性**: IP/CIDRベースのアクセス制御（allowed_sources）、資格情報露出防止。

## 3. アーキテクチャ
```
┌────────────────────────────────────────────┐
│ is03-relay (FastAPI)                      │
│                                            │
│  BLE Scan (ble.py) ──┐                     │
│  HTTP Ingest (/api/ingest) ─┐              │
│                             ▼              │
│                    RingBuffer+SQLite       │
│                    (db.py)                 │
│                             ▼              │
│                    Forwarder Queue         │
│                    (future: primary/sec)   │
│                                            │
│  REST (/api/status /api/events /api/logs)  │
│  WS (/ws)                                  │
│  UI (static/index.html)                    │
└────────────────────────────────────────────┘
```

### 3.1 プロセス構成
- 単一 FastAPI プロセス（uvicorn）。lifespanで DB 初期化、BLE スキャン、フラッシュタスク起動。
- systemd 管理（自動再起動、ログはjournal＋専用ログファイル）。

### 3.2 データパス
1. **BLE**: Bleakで常時スキャン、受信イベントを標準化→リング/キューへ。
2. **HTTP ingest**: `POST /api/ingest`（is02→is03互換ペイロード）→標準化→リング/キューへ。
3. **保存**: メモリリング(最新2000件)＋SQLite（上限2万件、古いもの削除）。10秒/50件単位でflush。
4. **転送**: Forwarder（将来クラウド/冗長先へPOST）。キュー上限5000、失敗時指数バックオフ。
5. **UI**: `GET /` でステータス/イベント/ログを表示。**WebSocketなし、HTTPポーリングのみ**（ブラウザ負荷抑制）。

## 4. 機能仕様
### 4.1 BLE/HTTP 受信
- BLE: MACを12桁HEX、rssi、manufacturer/service data をイベント化。lacisId生成規則は `3 + productType(3) + MAC12 + 0096` を踏襲。
- HTTP ingest: is02→is03互換フォーマット（gateway/sensor/state/raw/observedAt）。不足フィールドは null で埋め、リング/DBへ保存。

### 4.2 ストア
- ring_size=2000、max_db_size=20000、flush_interval=10s、flush_batch=50。
- 上限超過時はDB先頭を削除してローテーション。

### 4.3 Forwarder（再送準備）
- primary_url / secondary_url の順にPOST試行。
- 失敗連続時は指数バックオフ（base=2s, max=60s）。
- キュー上限5000、溢れ時は最古イベントを破棄。

### 4.4 API / UI / Access Control
- `/api/status`: hostname/uptime/ring/queue/lastFlush 等を返却。
- `/api/events?limit=N`: 最新イベント取得（UIはポーリングで取得）。
- `/api/logs?lines=N`: ログTail。
- `/api/ingest`: IP/CIDRフィルタ `allowed_sources` で制限可。
- `/api/config`: 設定取得/更新（将来拡張、現行は静的JSON）。
- `/`: 統合UI（ステータス/イベント/ログ/設定編集）。**UI表示も allowed_sources で制限**。WSは無効（/wsは410応答）。

### 4.5 設定
- `config.json` を `/etc/is03-relay/` に配置（例を同梱）。
- 主な項目: BLE有効/無効、データディレクトリ(`/var/lib/is03-relay`)、リング/DB上限、forwarder先URL、allowed_sources。

## 5. ディレクトリ構成（code/orangePi/is03-relay）
```
is03-relay/
├── app/
│   ├── main.py      # FastAPIエントリ
│   ├── db.py        # リング + SQLite + flush
│   ├── ble.py       # BLEスキャン
│   ├── web.py       # API/WS/静的配信
│   └── static/index.html
├── design/DESIGN.md # 本書
├── README.md
├── requirements.txt
└── is03-relay.service
```

## 6. 運用・デプロイ手順（要点）
1. Armbianイメージ書き込み → 起動 → ismsユーザー作成。
2. 固定IP設定（例: is03-1=192.168.96.201 / is03-2=192.168.96.202）。
3. 依存インストール: `apt install bluetooth bluez python3-venv ...`
4. 配置: `/opt/is03-relay` にコードと venv。`/etc/is03-relay/config.json` を配置。
5. systemd: `is03-relay.service` を `/etc/systemd/system/` へ配置し enable。
6. 確認: `curl http://<IP>:8080/api/status` / ブラウザで `/` を確認。

## 7. 既知の課題・今後の拡張
- BLEスキャンの信頼性とCPU負荷は実機検証が必要。必要ならスキャン間隔/フィルタを追加。
- WebSocketによるライブ配信は現行仮実装レベル。高頻度環境ではバッチ配信・サンプリングを検討。
- Forwarderは今後クラウド/冗長先URL確定後に実配備。TLS検証やリトライポリシーの調整が必要。
- 認証/認可はIPフィルタのみ。Basic/OAuth等が必要ならリバースプロキシ側で実装を検討。
