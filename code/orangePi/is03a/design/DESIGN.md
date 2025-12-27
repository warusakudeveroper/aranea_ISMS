# is03a - ローカル受信リレー (Orange Pi Zero3)

作成日: 2025-12-24  
目的: is01/02/04/05/06/10等のローカル冗長受信・モニタリング・簡易操作 (HTTP/MQTT) を Orange Pi で実現。ブラウザ負荷を抑えつつリアルタイム監視を提供。

## 1. 要求
- **ローカル受信**: BLEスキャン (is01) + HTTP ingest (is02/他デバイス) をリング/SQLiteへ蓄積。
- **冗長化・再送準備**: Forwarderキューでプライマリ/セカンダリへPOST（バックオフ・上限あり）。
- **遠隔操作**: MQTTコマンド受信 (ping / relay_now / set_config) と心拍/status pub。
- **管理UI**: ステータス・イベント・ログ・設定編集＋リアルタイムモニター（SSE/EventSource）を1画面で。
- **アクセス制御**: allowed_sources によるIP/CIDR制限。

## 2. システム概要
```
FastAPI(uvicorn)
  ├ ingest_http (/api/ingest) → normalize → store.enqueue → forwarder.enqueue → eventHub.broadcast
  ├ ingest_ble  (BleakScanner) → normalize → 同上
  ├ forwarder   (primary→secondary, backoff, max_queue)
  ├ store       (RingBuffer 2000 / SQLite 20000, flush 10s/50件)
  ├ mqtt        (asyncio-mqtt, command handlers, heartbeat pub)
  ├ eventHub    (SSE subscribers, backpressure: queue maxlen)
  ├ UI (/): status + table + SSE + config editor
  └ API: /api/status /api/events /api/events/stream /api/logs /api/config
```

## 3. 主要仕様
- productType/productCode: デフォルト `003` / `0096`（lacisId生成用）、device_type `is03a`。
- Forwarder: max_queue=5000, backoff_base=2s, backoff_max=60s, timeout=8s, max_retry=3。
- Store: ring_size=2000, max_db_size=20000, flush_interval=10s, flush_batch=50。
- SSE: `/api/events/stream` で最新イベントをpush（イベントHubで複数購読者を扱う）。
- Access: allowed_sources は UI/API/ingest/logs/config/events に適用。

## 4. 設定 (config.yaml 例)
```yaml
device:
  name: is03a
  device_type: is03a
  product_type: "003"
  product_code: "0096"
  data_dir: /var/lib/is03a
  log_dir: /var/log/is03a
  interface: end0

relay:
  primary_url: ""
  secondary_url: ""
  timeout_sec: 8
  max_retry: 3
  max_queue: 5000
  backoff_base_sec: 2
  backoff_max_sec: 60

mqtt:
  host: ""
  port: 8883
  tls: true
  user: ""
  password: ""
  base_topic: "aranea"

register:
  gate_url: "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
  tid: ""
  tenant_lacisid: ""
  tenant_email: ""
  tenant_cic: ""

store:
  ring_size: 2000
  max_db_size: 20000
  flush_interval_sec: 10
  flush_batch: 50

access:
  allowed_sources:
    - "192.168.0.0/24"
    - "10.0.0.5"
```

## 5. UI
- 1ページ構成: ステータスカード / ログTail / イベント表 / 設定フォーム / ライブモニター。
- リアルタイム: EventSource(SSE) で最新イベントを逐次追記。負荷低減のため行数上限(200)とフィルタを保持。
- 設定保存: `/api/config` POST → config_override.yaml に保存 → Forwarder/MQTT/Storeを動的反映（リング/DB上限変更は再起動推奨）。

## 6. 運用
- デプロイ: `/opt/is03a` に設置、venv作成、`is03a.service` を systemd 登録。
- 固定IP: is03-1=192.168.96.201 / is03-2=192.168.96.202（推奨）。
- 監視: `/api/status` で ring/queue/lastFlush、MQTT接続、uptime を確認。

## 7. 今後の拡張
- BLEフィルタ・サンプリング設定、イベント圧縮。
- Forwarder先TLS設定や署名、送信結果のUI表示。
- 認証（Basic/Token）導入、ログローテート/メトリクス。
