# is20s - Network Capture Gateway (Orange Pi Zero3)

**バージョン**: v0.9.0-beta (2024-12-31)
**ステータス**: ローカル機能安定β版
**GitHub**: https://github.com/anthropics/aranea_ISMS (private)

Armbian上でネットワークトラフィックをキャプチャし、ドメインサービス分類、CelestialGlobeへのIngest連携を行うエッジゲートウェイ。

---

## デプロイ済みデバイス一覧

### 本番テスト環境（現地配置済み）

| デバイス | ホスト名 | サブネット | WiFi IP | Wired IP | lacisId |
|----------|----------|-----------|---------|----------|---------|
| is20s-tam | is20s-tam | 125 | 192.168.125.248 | 192.168.125.249 | 302002004F750DAA0096 |
| is20s-to | is20s-to | 126 | 192.168.126.249 | 192.168.126.249 | 30200200C7FDB5550096 |

**ネットワーク構成**:
- WiFi: 管理・API用
- Wired: スイッチミラーポート直結（パケットキャプチャ用）

### ハードウェア仕様

| 項目 | 仕様 |
|------|------|
| SoC | Allwinner H618 (4-core Cortex-A53) |
| RAM | 4GB DDR4 |
| Storage | 64GB eMMC |
| Network | 1GbE Ethernet + WiFi |
| OS | Armbian 25.5.1 (kernel 6.12.23) |

### 認証情報

```
ユーザー: mijeosadmin
パスワード: mijeos12345@
```

---

## 現在の実装状況

### 完成している機能 ✓

| 機能 | 状態 | 備考 |
|------|------|------|
| パケットキャプチャ | ◎ | tshark連携、SYN_ONLYモード |
| ドメインサービス辞書 | ◎ | 1072エントリ、マッチング動作確認済 |
| 不明ドメイン収集 | ◎ | 閾値ベースで蓄積 |
| ローカルAPI | ◎ | FastAPI/uvicorn |
| Web UI | ◎ | リアルタイム表示 |
| ファイルログ（NDJSON） | ◎ | ローテーション動作確認済 |
| Ingest Client (Phase A) | ◎ | API実装完了、enabled=false |

### 未実装の機能 ✗

| 機能 | 状態 | 必要な作業 |
|------|------|-----------|
| lacisOauth認証 | × | AraneaRegister.cpp準拠の認証フロー実装 |
| araneaDeviceGate連携 | × | CIC取得APIクライアント実装 |
| CIC取得・保存 | × | 認証後のCIC永続化 |
| CelestialGlobe送信 | × | dry_run=true → false切替 |
| SSoT辞書同期 | × | 差分同期クライアント実装 |
| MQTT接続 | × | クラウド側エンドポイント待ち |

---

## パフォーマンス指標（2024-12-31 実測）

### 両デバイス共通

| 指標 | is20s-tam | is20s-to |
|------|-----------|----------|
| CPU温度 | 45.3°C | 42.3°C |
| メモリ使用 | 498MB / 3.8GB | 586MB / 3.2GB |
| CPU周波数 | 1416 MHz | 1416 MHz |
| 負荷平均 | 0.35 | 0.46 |
| drop_count | 0 | 0 |
| 辞書エントリ | 1072 | 1072 |

### 負荷テスト結果

```
テスト条件: 10同時HTTPリクエスト → /api/status

結果:
  最速: 16.7ms
  最遅: 45.3ms
  平均: 約30ms

評価: ✓ 問題なし（10同時でも全て50ms以下）
```

---

## セットアップ

### 開発環境

```bash
cd code/orangePi/is20s
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt

# PYTHONPATH を通す
export PYTHONPATH=$(pwd)/..:$(pwd)/../global
uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload
```

### 本番デプロイ

```bash
# ファイル転送
scp -r app/* mijeosadmin@<device_ip>:/opt/is20s/app/

# サービス再起動
ssh mijeosadmin@<device_ip> "sudo systemctl restart is20s.service"
```

### systemd設定

```bash
sudo cp is20s.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now is20s
```

---

## API リファレンス

### 基本API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/status` | GET | 全ステータス取得 |
| `/api/events` | GET | イベント取得（?limit=N） |
| `/api/config` | GET/POST | 設定取得/更新 |
| `/api/logs` | GET | ログ参照（?lines=200） |

### ドメインサービスAPI

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/domain-services` | GET | 辞書全件取得 |
| `/api/domain-services/register` | POST | パターン登録 |
| `/api/domain-services/unknown` | POST | 不明ドメイン取得 |
| `/api/domain-services/batch-register` | POST | 一括登録 |

### CelestialGlobe Ingest API（Phase A）

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/api/ingest/config` | GET/POST | Ingest設定取得/更新 |
| `/api/ingest/stats` | GET | 統計取得 |
| `/api/ingest/start` | POST | Ingest開始 |
| `/api/ingest/stop` | POST | Ingest停止 |
| `/api/ingest/flush` | POST | 即時送信 |

---

## CelestialGlobe連携仕様

### Ingestペイロード形式

```json
{
  "auth": {
    "tid": "T2025120608261484221",
    "lacisId": "302002004F750DAA0096",
    "cic": "***"
  },
  "report": {
    "sourceType": "ar-is20s",
    "capturedAt": "2024-12-31T12:00:00Z",
    "packets": [
      {
        "ts": "2024-12-31T12:00:00.123Z",
        "rid": "306",
        "srcIp": "192.168.125.100",
        "domain": "example.com",
        "service": "Example",
        "category": "Cloud"
      }
    ],
    "summary": {
      "totalPackets": 100,
      "uniqueDomains": 50,
      "topServices": [
        {"service": "YouTube", "count": 30}
      ]
    }
  }
}
```

### SSoT辞書同期（設計済み・未実装）

```
CelestialGlobe (SSoT)
    ↓ 差分同期
IS-20S (ローカル辞書)
    ↓ 検出
未知ドメイン
    ↓ 報告
CelestialGlobe
    ↓ Metatron AI判定
辞書更新 → 全IS-20Sへ同期
```

---

## 運用注意事項

### eMMC書き込み保護

現在の実装では `capture_logs/*.ndjson` が毎パケットでファイル操作を行うため、長期運用でeMMC劣化のリスクあり。

**対策（将来実装）**:
- capture_logsをtmpfsに移行
- 定期永続化（10分間隔）

### 発熱対策

CPU温度が65°C超の場合、最大クロックを1200MHzに制限推奨：

```bash
echo 1200000 | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq
```

---

## SDK ナレッジ参照

以下のナレッジがaranea-sdkに登録済み：

| ID | タイトル |
|----|----------|
| IOi7IYMZj1gRe25zmDGg | IS-20S 今後の検討課題・ロードマップ v2 |
| TGSMzKIdF68z0XuMOlsv | SSoT辞書 差分同期設計仕様 |
| 5vD3ZUqv0jpPxqnPZIJ8 | IS-20S (Orange Pi Zero3) パフォーマンス最適化・発熱対策ガイド |
| ol0IgEsDRnklfxYvs5z6 | IS-20S ドメインサービス辞書 設計・運用ベストプラクティス |
| VV26OOxHfsPnbJoxkDSL | Metatron AI ドメイン判定ルール |

---

## 変更履歴

| 日付 | バージョン | 変更内容 |
|------|-----------|---------|
| 2024-12-31 | v0.9.0-beta | CelestialGlobe Ingest Client Phase A追加 |
| 2024-12-29 | v0.8.0 | ドメインサービス辞書機能追加 |
| 2024-12-24 | v0.7.0 | 初期リリース |
