# 03 状態レポート（deviceStateReport / ローカル）詳細

## エンドポイント
- クラウド（現行）：`https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport`
- ローカル中継：relay_pri / relay_sec（UI `/settings` で設定）

## ペイロード（is05a 実装）
### ローカル送信（中継向け）
```json
{
  "observedAt": "ISO8601",
  "sensor": { "lacisId": "<device>", "mac": "<MAC>", "productType": "005", "productCode": "0001" },
  "state": { "ch1": "OPEN/CLOSE", ..., "ch8_lastUpdatedAt": "timestamp", "rssi": "-60", "ipaddr": "192.168.x.x", "SSID": "..." },
  "meta": { "observedAt": "...", "direct": true },
  "gateway": { "lacisId": "<device>", "ip": "...", "rssi": -60 }
}
```
### クラウド送信（deviceStateReport）
```json
{
  "auth": { "tid": "<tenantId>", "lacisId": "<device>", "cic": "<cic>" },
  "report": {
    "lacisId": "<device>",
    "type": "aranea_ar-is05a",
    "observedAt": "ISO8601",
    "state": { "ch1": "OPEN/CLOSE", ..., "ch8": "OPEN/CLOSE" }
  }
}
```

## 実装パラメータ
- HTTP timeout 3000ms、最大連続失敗 3 回で 30 秒バックオフ、最小送信間隔 1s。Wi-Fi 未接続はスキップ。
- stateEndpoint は register 応答を優先。UI 設定値との差分を CLI で検出予定。

## 課題
- productCode: 実装は `0001` 固定。仕様/カタログ `0096` への統一要否を決定し、移行ステップを準備。
- typeId: 送信値 `aranea_ar-is05a` を registry 正式名（例: `ar-is05`）と揃える。

## CLI/SDK での要求
- `cli/report test --payload fixtures/... --endpoint http|mqtt` で送信→応答コード・再試行を記録。
- Firestore/BQ 反映確認は `cli/report verify-firestore/bq` で実行（詳細は 06_testing_and_monitoring）。
