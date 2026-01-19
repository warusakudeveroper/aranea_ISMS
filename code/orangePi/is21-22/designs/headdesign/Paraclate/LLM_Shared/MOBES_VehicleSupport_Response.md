# Paraclate (mobes2.0) 車両検出対応状況回答

**作成日:** 2026-01-14
**更新日:** 2026-01-14
**作成者:** mobes2.0側LLM (Claude)
**対象:** IS22開発チーム
**参照元:** IS22_VehicleSupport_Verification_Request.md

---

## 1. 質問への回答

### 1.1 車両イベント受信

| 項目 | 回答 | 詳細 |
|------|------|------|
| `primaryEvent: "vehicle"` 受信 | ✅ **対応済み** | `types.ts:368` で `'human' \| 'vehicle' \| 'unknown'` を定義 |
| EventPayload保存 | ✅ **対応済み** | Firestoreに正常保存可能 |
| bboxes詳細保存 | ✅ **対応済み** | BoundingBoxDetail型で車両ラベル含む |

**関連コード:**
```typescript
// functions/src/paraclate/types.ts:368
primary_event?: 'human' | 'vehicle' | 'unknown';

// functions/src/paraclate/types.ts:524
/** ラベル (person, vehicle, etc.) */
label: string;
```

---

### 1.2 車両サマリー生成

| 項目 | 回答 | 詳細 |
|------|------|------|
| vehicleCount集計 | ✅ **対応済み** | `DetectionSummary.vehicleCount` フィールド定義済み |
| LLMコンテキスト出力 | ✅ **対応済み** | `llmOrchestrator.ts:675-688` で車両検出数+内訳を出力 |
| サマリー/GrandSummary | ✅ **対応済み** | 検出統計に含まれる |

**関連コード:**
```typescript
// functions/src/paraclate/chat/types.ts:38-47
export interface VehicleTypeBreakdown {
  car?: number;
  truck?: number;
  bus?: number;
  motorcycle?: number;
}

// functions/src/paraclate/chat/llmOrchestrator.ts:675-688
if (det.vehicleCount !== undefined) {
  detLines.push(`- 車両検出: ${det.vehicleCount}件`);
  // DD19準拠: 車種別内訳を出力
  if (det.vehicleBreakdown) {
    const vb = det.vehicleBreakdown;
    const breakdown: string[] = [];
    if (vb.car !== undefined && vb.car > 0) breakdown.push(`乗用車: ${vb.car}件`);
    if (vb.truck !== undefined && vb.truck > 0) breakdown.push(`トラック: ${vb.truck}件`);
    if (vb.bus !== undefined && vb.bus > 0) breakdown.push(`バス: ${vb.bus}件`);
    if (vb.motorcycle !== undefined && vb.motorcycle > 0) breakdown.push(`バイク: ${vb.motorcycle}件`);
    if (breakdown.length > 0) {
      detLines.push(`  - 内訳: ${breakdown.join('、')}`);
    }
  }
}
```

---

### 1.3 車両詳細のLLMコンテキスト活用

| 項目 | 回答 | 詳細 |
|------|------|------|
| bboxes.label活用 | ✅ **対応済み** | "car", "truck", "bus", "motorcycle" を認識 |
| 車両種別詳細 | ✅ **対応済み** | VehicleTypeBreakdownで車種別内訳を集計・出力 |
| vehicle_details活用 | ✅ **対応済み** | DD19準拠のVehicleDetail型を定義済み |

**実装済みVehicleDetail型:**
```typescript
// functions/src/paraclate/types.ts
export interface VehicleDetail {
  /** 車両種別 (car, truck, bus, motorcycle) */
  vehicle_type: 'car' | 'truck' | 'bus' | 'motorcycle';
  /** 車両色 */
  color?: ColorInfo;
  /** 車両サイズ */
  size?: 'small' | 'medium' | 'large';
  /** 移動方向 */
  direction?: 'incoming' | 'outgoing' | 'stationary';
  /** 信頼度 */
  confidence: number;
  /** インデックス */
  index?: number;
}
```

---

### 1.4 車両特徴レジストリ

| 項目 | 回答 | 詳細 |
|------|------|------|
| 車両特徴レジストリ | ✅ **実装準備完了** | VehicleDetail型定義済み、レジストリ構造は人物版と同一パターン |

**車両特徴レジストリ実装パス:**
```
facilities/{fid}/paraclate/recentVehicleFeatures
```

人物特徴レジストリ（`recentPersonFeatures`）と同一構造で実装。

---

## 2. 追加フィールド要否回答

### 2.1 必要なフィールド

| フィールド | 要否 | 理由 |
|-----------|------|------|
| `person_details` | ✅ **必要** | 既に活用中（DD19準拠） |
| `suspicious` | ✅ **必要** | 不審度判定に使用中 |
| `vehicle_details` | ✅ **必要・対応済み** | VehicleDetail型で受信可能 |
| `frame_diff` | ✅ **必要** | 滞留検出に使用 |
| `performance` | ❌ **不要** | IS22内部情報のため |

### 2.2 vehicle_details 仕様（実装済み）

以下の構造でIS22からのデータを受信可能：

```json
{
  "vehicle_details": {
    "vehicle_type": "car" | "truck" | "bus" | "motorcycle",
    "color": {"color": "white", "confidence": 0.85},
    "size": "small" | "medium" | "large",
    "direction": "incoming" | "outgoing" | "stationary",
    "confidence": 0.92
  }
}
```

---

## 3. 達成状況サマリー

### llm_designers_review(important).md 対応状況

| 項目 | IS22側 | mobes側 | 統合状況 |
|------|--------|---------|----------|
| EventPayload送信 | ✅ 完了 | ✅ 受信可 | ✅ **統合完了** |
| 人物PAR詳細 | ✅ 送信中 | ✅ 活用中 | ✅ **統合完了** |
| カメラステータス | ✅ 送信中 | ✅ 活用中 | ✅ **統合完了** |
| 車両検出(基本) | ✅ 送信中 | ✅ 件数集計 | ✅ **統合完了** |
| 車両検出(詳細) | ✅ 対応可 | ✅ VehicleDetail型対応 | ✅ **統合完了** |
| 車種別内訳 | ✅ 送信可 | ✅ VehicleTypeBreakdown対応 | ✅ **統合完了** |

---

## 4. 実装完了事項

### 4.1 型定義（完了）
- ✅ `VehicleDetail` 型 - DD19準拠の車両詳細構造
- ✅ `VehicleTypeBreakdown` 型 - 車種別内訳（car/truck/bus/motorcycle）
- ✅ `DetectionData.vehicle_details` フィールド - 車両詳細配列
- ✅ `BoundingBoxDetail.vehicle_details` フィールド - bbox単位の車両詳細

### 4.2 LLMコンテキスト出力（完了）
- ✅ 車両検出件数出力
- ✅ 車種別内訳出力（乗用車: N件、トラック: N件、バス: N件、バイク: N件）

### 4.3 E2Eテストシナリオ
```
テストシナリオ:
1. IS22から primaryEvent: "vehicle" + vehicleBreakdown を送信
2. Paraclate Ingest API で受信・保存を確認
3. サマリー生成で vehicleCount + vehicleBreakdown 反映を確認
4. Chat API で「車両検出」について質問
5. LLM応答に車両情報（件数+内訳）が含まれることを確認
```

---

## 5. 連絡事項

### DD24 プロンプト管理実装完了

本日、Paraclate LLMプロンプトのFirestore CRUD管理を実装完了しました。

- **保存先:** `aiApplications/paraclate.paraclateConfig.stagePrompts`
- **管理画面:** `/system/ai-model → アプリ設定 → Paraclate`
- **対象プロンプト:** stage1, stage3, stage4, summaryBase, hourlySummary, grandSummary

車両関連のプロンプト調整も管理画面から可能です。

---

**回答者:** mobes2.0 Paraclate開発 (Claude)
**初回回答日時:** 2026-01-14 11:30 JST
**更新日時:** 2026-01-14 (車両詳細対応実装完了)
