# 02 araneaDeviceGate（登録）詳細

## エンドポイント
- URL（現行）：`https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`
- 保存先：NVS/ローカル設定。UI `/tenant` から上書き可能。

## リクエスト（実装正: `AraneaRegister.cpp`）
```json
{
  "lacisOath": { "lacisId": "...", "userId": "...", "cic": "...", "method": "register" },
  "userObject": {
    "lacisID": "<device lacisId>",
    "tid": "<tenantId>",
    "typeDomain": "araneaDevice",
    "type": "<deviceType e.g. ISMS_ar-is05a>"
  },
  "deviceMeta": {
    "macAddress": "<MAC12HEX>",
    "productType": "<3-digit>",
    "productCode": "<4-digit>"
  }
}
```

## レスポンス処理
- 200/201: `userObject.cic_code`, `stateEndpoint`, `mqttEndpoint?` を保存。
- 409: MAC 重複 → 中断。
- JSON parse 失敗や HTTP エラー時は再登録（`clearRegistration`）を推奨。
- 既登録フラグあり + CIC 空なら再登録を自動実行。

## 差分/課題
- productCode は実装で `0001` 固定、仕様/カタログは `0096`。正本決定と移行手順が必要。
- type 名称: 実装送信は `aranea_ar-is05a` 等。Type registry の正式 `typeId`（例: `ar-is05`）と整合させる。
- 追加予定: ownershipChanged/ETag/楽観ロックの応答を受け取った場合は保存・表示できるように拡張。

## CLI/SDK での要求
- `cli/register` で上記 JSON を生成し、ETag/楽観ロックに対応（将来 API 化時）。
- `cli/register test` はダミー登録→レスポンス/保存値/NVS（またはファイル）を検証。
