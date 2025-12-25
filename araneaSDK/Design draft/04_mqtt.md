# 04 MQTT / WebSocket 設計とテスト

## 想定トピック（案）
- Publish: `aranea/{tid}/{lacisId}/state`
- Subscribe: `aranea/{tid}/{lacisId}/command`
- QoS: 1 を基本、LWT を必須化。

## 認証/接続
- 認証トークン: lacisOath 派生または MQTT 専用資格（mobes 側で発行想定）。
- エンドポイント: register 応答 `mqttEndpoint` を正本とし、未提供の場合は HTTP のみ運用。
- Web（araneawebdevice）は MQTT over WebSocket を優先。WS 不可の場合は HTTP へフォールバック。

## テスト機能
- `cli/mqtt echo --env dev --topic ... --payload ...`  
  - pub→sub 往復確認、TLS/WS、LWT、再接続、throughput を計測。
- 障害試験: 強制切断→再接続、LWT 配信確認、バースト送信によるスループット測定。
- トピック衝突検出: Type registry に記載された topic と比較し、複数 type での重複を警告。

## 実装ガイド
- 各ランタイムで共通 API: `mqtt.connect(endpoint, topics, opts)` を提供し、opts に LWT/keepalive/QoS/WS を含める。
- MQTT が未稼働の場合、CLI は HTTP モードのみで PASS とし、警告を表示。
