# 05 Type 登録・スキーマ同期

## データモデル（Firestore 想定: araneaDeviceConfig）
```json
{
  "typeDomain": "araneaDevice",
  "productType": "005",
  "productCode": "0096",
  "typeId": "ar-is05",
  "displayName": "is05 8ch sensor",
  "stateSchema": { "$ref": "fixtures/type/ar-is05.state.schema.json" },
  "commandSchema": { "$ref": "fixtures/type/ar-is05.command.schema.json" },
  "mqtt": { "pubTopic": "aranea/{tid}/{lacisId}/state", "subTopic": "...", "qos": 1 },
  "telemetry": { "requiredFields": ["state.ch1", "state.lastUpdatedAt"], "maxReportIntervalSec": 90 },
  "version": 3,
  "deprecated": false,
  "supersedes": ["ar-is05@2"],
  "hash": "sha256:xxxx",
  "updatedBy": "lacisId-of-editor",
  "updatedAt": "2025-01-21T12:00:00Z",
  "notes": "breaking change: added ch8"
}
```

## API/アクセス
- HTTP（将来 API）: GET 一覧/単体, POST 新規, PUT 更新（ETag 必須）。
- Firestore 直接（暫定・管理者のみ）: `updateTime` による楽観ロック。
- 認証: lacisOath JWT（scope: type.read/write）、CI 用 SA トークン。

## CLI コマンド
- `cli/type fetch --env dev --domain araneaDevice --out fixtures/type/`
- `cli/type diff --local ... --remote ...`（breaking 判定付き）
- `cli/type register --file ... --env dev --force-breaking`
- `cli/type promote --type ... --from dev --to stg|prod --allow-deprecated`
- `cli/type lint --file ...`
- `cli/type lock --type ... --env prod --message ...`

## バージョニング/ガード
- version は単調増加。breaking 変更は `--force-breaking` が必須。
- productType/productCode の再利用禁止チェック。MQTT topic 衝突チェック。
- hash = sha256(stateSchema + commandSchema + meta canonical)。変更がなければ再登録をスキップ。

## fixtures
- `fixtures/type/{typeId}.state.schema.json` / `{typeId}.command.schema.json` を正本とし、`{typeId}.local.json` で meta/hash を管理。
