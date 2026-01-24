# IS06S 設計仕様乖離分析

**作成日**: 2026-01-24
**対象**: DesignerInstructions.md vs 実装
**ステータス**: 実装未完了 - 乖離あり

---

## 1. 乖離サマリ

| カテゴリ | 設計項目数 | 実装済み | 未実装/不完全 | 乖離率 |
|----------|-----------|---------|--------------|-------|
| PIN機能 (Backend) | 12 | 10 | 2 | 17% |
| WebUI | 8 | 2 | 6 | 75% |
| HTTP API | 6 | 4 | 2 | 33% |
| **合計** | **26** | **16** | **10** | **38%** |

---

## 2. 詳細乖離リスト

### 2.1 WebUI - PIN Control タブ

| 設計仕様 | 実装状況 | 乖離内容 |
|----------|----------|----------|
| stateName表示 (Digital) | ❌ 未実装 | 設計: `["on:解錠","off:施錠"]` → 実装: 固定"ON"/"OFF" |
| stateName表示 (PWM) | ❌ 未実装 | 設計: `["0:0%","30:30%",...]` → 実装: 数値%のみ |
| PIN name表示 | ⚠️ 不完全 | 構造あり、表示は"CH1"固定 |

**該当コード (HttpManagerIs06s.cpp:91-98):**
```javascript
// 現状: 固定テキスト
ctrl = '<button ...>' + (st ? 'ON' : 'OFF') + '</button>';

// 設計要件: stateNameから取得すべき
// 例: st ? '解錠' : '施錠'
```

### 2.2 WebUI - PIN Settings タブ

| 設計仕様 | 実装状況 | 乖離内容 |
|----------|----------|----------|
| 設定保存 | ❌ 未実装 | `alert('Settings saved (not implemented yet)')` |
| stateName設定 | ❌ 未実装 | UI入力欄なし |
| allocation設定 (I/O Type) | ❌ 未実装 | 連動先CH選択UIなし |
| expiryDate設定 | ❌ 未実装 | 有効期限入力UIなし |
| debounce設定 | ❌ 未実装 | フォームに項目なし |
| rateOfChange設定 | ❌ 未実装 | フォームに項目なし |

**該当コード (HttpManagerIs06s.cpp:161-163):**
```javascript
function savePinSettings() {
  alert('Settings saved (not implemented yet)');  // ← 未実装
}
```

### 2.3 HTTP API - POST /api/pin/{ch}/setting

| 設計仕様 | 実装状況 | 乖離内容 |
|----------|----------|----------|
| name保存 | ❌ 未実装 | TODO コメントのみ |
| actionMode保存 | ❌ 未実装 | TODO コメントのみ |
| validity保存 | ❌ 未実装 | TODO コメントのみ |
| debounce保存 | ❌ 未実装 | TODO コメントのみ |
| rateOfChange保存 | ❌ 未実装 | TODO コメントのみ |
| stateName保存 | ❌ 未実装 | TODO コメントのみ |
| allocation保存 | ❌ 未実装 | TODO コメントのみ |
| expiryDate保存 | ❌ 未実装 | TODO コメントのみ |

**該当コード (HttpManagerIs06s.cpp:346):**
```cpp
// TODO: その他の設定 (name, actionMode, validity, etc.)
```

### 2.4 Backend - Is06PinManager

| 設計仕様 | 実装状況 | 乖離内容 |
|----------|----------|----------|
| name NVS永続化 | ❌ 未実装 | loadFromNvs/saveToNvsに未実装 |
| stateName NVS永続化 | ❌ 未実装 | 構造体にあるがNVS保存なし |
| actionMode NVS永続化 | ⚠️ 不完全 | デフォルト設定のみ、保存なし |
| allocation NVS永続化 | ❌ 未実装 | 構造体にあるがNVS保存なし |

---

## 3. 実装が必要な項目（優先順位順）

### P1: 必須 - 実用不可（設計仕様の中核機能）

| ID | 項目 | 影響範囲 | 工数目安 |
|----|------|----------|----------|
| P1-1 | PIN Settings 保存機能 | API + WebUI | 中 |
| P1-2 | stateName 表示・設定 | API + WebUI + Backend | 大 |
| P1-3 | allocation 設定 (I/O Type) | API + WebUI + Backend | 中 |
| P1-4 | actionMode 保存・読込 | API + Backend | 小 |
| P1-5 | validity/debounce/rateOfChange 設定 | API + WebUI | 中 |

### P2: 必須 - 機能制限あり

| ID | 項目 | 影響範囲 | 工数目安 |
|----|------|----------|----------|
| P2-1 | expiryDate 設定UI | WebUI | 中 |
| P2-2 | name 表示・設定 | API + WebUI + Backend | 小 |
| P2-3 | PWMプリセット設定 | API + WebUI | 中 |

---

## 4. 設計仕様との対照表

### 4.1 PINSettings (DesignerInstructions.md)

```json
"PINSettings":{
  "CH2":{
    "type":"digitalOutput",      // ✅ 実装済み
    "name":"照明スイッチ",         // ❌ 保存未実装
    "stateName":["on:解錠","off:施錠"], // ❌ 未実装
    "actionMode":"Mom",           // ⚠️ 読込のみ、保存未実装
    "defaultValidity":"3000",     // ⚠️ API保存未実装
    "defaultDebounce":"3000",     // ⚠️ API保存未実装
    "defaultexpiry":"1"           // ⚠️ API保存未実装
  }
}
```

### 4.2 I/O Type allocation (DesignerInstructions.md)

```json
"PINSettings":{
  "CH6":{
    "type":"digitalInput",        // ✅ 実装済み
    "allocation":["CH1","CH2"],   // ❌ 設定UI未実装
    "triggerType":"Digital",      // ⚠️ 推論のみ
    "actionMode":"Mom"            // ⚠️ 保存未実装
  }
}
```

---

## 5. Golden Rules 違反事項

| Rule | 違反内容 |
|------|----------|
| Rule 6 | `savePinSettings()` が `alert('not implemented')` - 現場猫案件 |
| Rule 7 | stateName等を「低優先度」として棚上げ |
| Rule 9 | WebUI設定保存のテストなしに完了報告 |
| Rule 11 | 「構造体はある」で実用性問題を矮小化 |

---

## 6. 是正アクション

### 即時対応が必要

1. **IMPLEMENTATION_REPORT.md を削除または「未完了」に修正**
2. **タスクリストを更新し、未実装項目を追加**
3. **P1項目を順次実装**

### 実装順序（依存関係考慮）

```
P1-4 (actionMode保存)
  → P1-5 (validity等設定)
    → P1-1 (Settings保存機能)
      → P1-2 (stateName)
        → P1-3 (allocation)
          → P2-* (その他)
```

---

## 7. 結論

**現状は実用不可能です。**

WebUI PIN Settings タブの保存機能が `alert('not implemented')` であり、
設計仕様の中核機能である stateName, allocation の設定ができません。

レビュー依頼は上記乖離を解消後に実施すべきです。
