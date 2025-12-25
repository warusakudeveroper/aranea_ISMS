# 07 マルチランタイム共通モジュール + araneawebdevice

## ターゲットと役割
- ESP32: Arduino C++（既存 `AraneaRegister`, `StateReporter*` を SDK 内包）
- Linux SBC: Python / Node.js / 将来 Rust
- Web: araneawebdevice（ブラウザ/ServiceWorker、MQTT over WS）

## 共通 API サーフェス（全言語共通）
- `register(auth, userObject, deviceMeta)` → cic/stateEndpoint/mqttEndpoint
- `report(auth, payload)` → HTTP/MQTT 送信、再試行/バックオフ
- `mqtt.connect(endpoint, topics, opts)` → TLS/WS, LWT, reconnect
- `schema.validate(typeId, payload)` → JSON Schema バリデーション

## 配置案
```
lib/
  arduino/   (auth, register, report, mqtt stub)
  python/    (auth, register, report, mqtt client)
  node/      (auth, register, report, mqtt client)
  rust/      (auth/report traits, mqtt client via rumqttc)   // PoC/将来
  web/       (auth/report via fetch, mqtt via WebSocket/MQTT.js)
```

## ビルド/配布
- Node: npm `@aranea/sdk-node`
- Python: PyPI `aranea-sdk`
- Rust: crates.io（将来）
- ESP32: ライブラリ同梱 or git submodule

## araneawebdevice（Web 実装）
- 目的: Web システム内で仮想 araneaDevice を実装し、同一スキーマで送受信・デバッグ。
- 認証: lacisOath JWT もしくは短期トークン。CORS/HTTPS 前提。
- キャッシュ: ServiceWorker でバッファリングし、再接続時に送信。
- 整合: ESP32/Linux と同じ typeId/stateSchema/commandSchema を使用し、payload hash を比較する CI を実施。
