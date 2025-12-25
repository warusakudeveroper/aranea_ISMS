# 01 認証（lacisOath）多系統設計

## 方針
- 認証は ESP32（Arduino C++）と Linux 系（Python/Node, 将来 Rust）で二系統必須。Web/araneawebdevice も同じ JSON を使用。
- 正本は lacisOath（lacisId + userId + CIC + method）。パスワードは廃止。キー名は全言語で統一。

## ペイロード例（共通）
```json
{
  "lacisOath": {
    "lacisId": "<tenantPrimary lacisId>",
    "userId": "<tenantPrimary email>",
    "cic": "<tenantPrimary CIC>",
    "method": "register"
  }
}
```

## 実装モジュール
- Arduino: 既存 `AraneaRegister` 内に組み込み（NVS 保持、再登録時の clearRegistration を提供）。
- Python/Node（Linux SBC）: `lib/auth/python`, `lib/auth/node` を用意し、同じ JSON 構築 + ETag/再試行/トークンキャッシュを実装。
- Rust（将来）: trait で register/report の共通シグネチャを定義し、rumqttc などを利用。
- Web/araneawebdevice: fetch/MQTT over WS で同じ JSON を送信。短期トークンまたは lacisOath JWT を使用。

## 環境ファイルテンプレ
- `.env.esp32`: `TID`, `TENANT_LACISID`, `TENANT_EMAIL`, `TENANT_CIC`, `GATE_URL`, `STATE_URL`, `MQTT_URL?`
- `.env.linux`: 上記 + `MQTT_TOKEN` など Linux 専用の資格情報
- CLI は実行時にどの資格を使ったかを必ずログ出力（検証比較用）。
