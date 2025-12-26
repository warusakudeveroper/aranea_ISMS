# AraneaSDK Schemas

mobes2.0 Firestoreの正本スキーマをミラーリングしたJSON Schemaファイル群です。

## ディレクトリ構造

```
schemas/
├── README.md                        # このファイル
├── userObject.schema.json          # デバイス登録情報
├── deviceStateReport.schema.json   # 状態レポートリクエスト
└── types/
    ├── aranea_ar-is01.json         # 温湿度センサー
    ├── aranea_ar-is04a.json        # 接点コントローラー
    ├── aranea_ar-is05a.json        # 8ch検出器
    ├── aranea_ar-is06a.json        # 6ch出力
    └── aranea_ar-is10.json         # ルーター検査
```

## 使用方法

### スキーマ検証 (Node.js)

```javascript
const Ajv = require('ajv');
const schema = require('./types/aranea_ar-is04a.json');

const ajv = new Ajv();
const validate = ajv.compile(schema.stateSchema);

const state = {
  output1: { active: true },
  output2: { active: false }
};

if (validate(state)) {
  console.log('Valid!');
} else {
  console.error(validate.errors);
}
```

### ESP32での検証

```cpp
#include <ArduinoJson.h>

// stateSchema に基づく検証
bool validateIs04aState(const JsonObject& state) {
  if (!state.containsKey("output1") || !state.containsKey("output2")) {
    return false;
  }
  if (!state["output1"]["active"].is<bool>()) {
    return false;
  }
  return true;
}
```

## 同期ルール

1. **正本**: mobes2.0 Firestore `typeSettings/araneaDevice/{type}`
2. **ミラー**: このディレクトリ
3. **同期タイミング**: スキーマ変更時に手動同期

## Type別スキーマ構造

各Typeスキーマファイルは以下の構造を持ちます：

```json
{
  "metadata": {
    "productType": "...",
    "productCodes": [...],
    "displayName": "...",
    "capabilities": [...],
    "semanticTags": [...],
    "features": [...]
  },
  "stateSchema": { ... },
  "configSchema": { ... },
  "commandSchema": { ... }
}
```

## ProductType一覧

| ProductType | Type | Description |
|-------------|------|-------------|
| 001 | aranea_ar-is01 | 電池式温湿度センサー |
| 004 | aranea_ar-is04a | 2ch接点コントローラー |
| 005 | aranea_ar-is05a | 8ch検出器 |
| 006 | aranea_ar-is06a | 6ch出力 |
| 010 | aranea_ar-is10 | ルーター検査 |

## 関連ドキュメント

- [SCHEMA_SPEC.md](../Design/SCHEMA_SPEC.md) - スキーマ仕様詳細
- [TYPE_REGISTRY.md](../Design/TYPE_REGISTRY.md) - Type登録仕様
- [INTEGRATED_MASTER_PLAN.md](../Design/INTEGRATED_MASTER_PLAN.md) - 統合マスタープラン
