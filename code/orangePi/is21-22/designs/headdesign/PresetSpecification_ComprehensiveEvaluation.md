# プリセット仕様 包括的評価報告書

**Issue**: #107 プリセットグラフィカルUI・過剰検出判定
**作成日**: 2026-01-09
**評価者**: Claude Code
**対象システム**: is22 RTSPカメラ総合管理サーバー

---

## 1. エグゼクティブサマリー

### 1.1 評価概要

IS22システムには12種類のプリセットが定義されており、監視カメラの用途に応じたYOLO推論パラメータとフィルタリング設定を提供する。本報告書では各プリセットの設計妥当性、仕様の整合性、および改善提案を包括的に評価する。

### 1.2 総合評価

| 評価項目 | スコア | 備考 |
|----------|--------|------|
| 設計思想の一貫性 | ★★★☆☆ | 一部プリセットで設計方針に矛盾 |
| パラメータ設定の妥当性 | ★★★★☆ | 閾値設定は概ね適切 |
| 除外リストの網羅性 | ★★★☆☆ | 室内系プリセットで不足 |
| ユースケースカバレッジ | ★★★★☆ | 主要シーンは網羅 |
| 拡張性 | ★★★★★ | カスタムプリセット対応済 |

---

## 2. プリセット設計の基本パラメータ

### 2.1 パラメータ定義

| パラメータ | 説明 | 有効範囲 |
|-----------|------|----------|
| `conf_override` | YOLO検出信頼度閾値 | 0.2〜0.8（低=検出漏れ減、高=誤検知減） |
| `nms_threshold` | Non-Maximum Suppression閾値 | 0.3〜0.6（低=重複排除強、高=重複許容） |
| `par_threshold` | 人物属性認識閾値 | 0.3〜0.8（低=属性多取得、高=確実性重視） |
| `expected_objects` | 検出期待オブジェクト | human, vehicle, animal等 |
| `excluded_objects` | 検出除外オブジェクト | YOLO COCOラベル |
| `distance` | カメラ距離設定 | near, medium, far |
| `enable_frame_diff` | フレーム差分解析 | true/false |
| `suggested_interval_sec` | 推奨ポーリング間隔 | 秒数 |

### 2.2 YOLOv8 COCOラベル参考

監視カメラ環境で頻出するラベル:

| カテゴリ | ラベル | 備考 |
|----------|--------|------|
| 人物 | person | 主要検出対象 |
| 車両 | car, truck, bus, motorcycle, bicycle | 駐車場・屋外 |
| 動物 | dog, cat, bird, horse, cow, sheep | 屋外 |
| オフィス用品 | **mouse**, keyboard, laptop, cell phone, tv | 室内誤検知の主因 |
| 家具・什器 | chair, couch, bed, dining table | 通常は無害 |
| 雑貨 | book, clock, vase, scissors, remote | 室内誤検知要因 |

> **重要**: `mouse`はコンピューターマウス（オフィス用品）であり、動物のネズミ（rat）ではない。COCOデータセットにratは含まれない。

---

## 3. 全12プリセット詳細評価

### 3.1 balanced（バランス）- デフォルト推奨

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 汎用的な監視 | |
| **version** | 1.1.0 | |
| **distance** | medium | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | 14項目（mouse, keyboard等） | ✅ 最も充実 |
| **conf_override** | None（デフォルト） | ✅ 適切 |
| **par_threshold** | None | ✅ 適切 |
| **interval** | 15秒 | ✅ 適切 |

**評価: ★★★★★ 優秀**

最も安全なプリセット。室内オフィス用品を網羅的に除外しており、Unknown乱発リスクが最も低い。初期設定や問題発生時のフォールバック先として最適。

**改善提案**: なし（現状で最も完成度が高い）

---

### 3.2 person_priority（人物優先）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 人物検知最優先 | |
| **version** | 1.0.0 | |
| **distance** | medium | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | vehicle, animal | ⚠️ 不十分 |
| **conf_override** | 0.3（低め） | ✅ 人物逃さず |
| **par_threshold** | 0.4 | ✅ 詳細属性取得 |
| **output_schema** | person_detailed | ✅ 適切 |
| **interval** | 10秒 | ✅ 適切 |

**評価: ★★★☆☆ 要改善**

**問題点**:
- 室内設置時にmouse, keyboard等が検出されUnknown判定される
- 低conf閾値（0.3）により誤検知が増加する可能性

**改善提案**:
```rust
excluded_objects: vec![
    "vehicle", "animal",
    "mouse", "keyboard", "laptop", "cell phone",  // 追加
]
```

---

### 3.3 parking（駐車場）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 駐車場・駐輪場 | |
| **version** | 1.0.0 | |
| **distance** | far | ✅ 適切 |
| **expected_objects** | human, vehicle | ✅ 適切 |
| **excluded_objects** | 空 | ⚠️ 検討余地あり |
| **conf_override** | 0.45 | ✅ 遠距離対応 |
| **interval** | 20秒 | ✅ 適切 |

**評価: ★★★★☆ 良好**

**分析**:
- 屋外駐車場では室内オブジェクトは検出されないため、空の除外リストは許容範囲
- ただし、屋内駐車場（ビル地下等）では管理室のPC類が映り込む可能性あり

**改善提案**:
- 屋内駐車場向けサブプリセット `parking_indoor` の追加を検討
- または、excluded_objectsに最低限のオフィス用品を追加

---

### 3.4 entrance（エントランス）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 建物入口 | |
| **version** | 1.0.0 | |
| **distance** | near | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | vehicle | ⚠️ 不十分 |
| **conf_override** | 0.35 | ✅ 近距離対応 |
| **par_threshold** | 0.4 | ✅ 適切 |
| **interval** | 10秒 | ✅ 適切 |

**評価: ★★★☆☆ 要改善**

**問題点**:
- エントランスには受付デスクがあることが多く、mouse/keyboard検出リスク
- cell phoneを持った来訪者が頻繁に検出される可能性

**改善提案**:
```rust
excluded_objects: vec![
    "vehicle",
    "mouse", "keyboard", "cell phone",  // 追加
]
```

---

### 3.5 corridor（廊下）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 廊下・通路 | |
| **version** | 1.1.0 | ✅ 修正済 |
| **distance** | medium | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | 8項目（修正後） | ✅ 改善済 |
| **conf_override** | None | ✅ 適切 |
| **interval** | 15秒 | ✅ 適切 |

**評価: ★★★★☆ 良好（修正後）**

**修正内容（v1.1.0）**:
- 空だったexcluded_objectsに室内共通除外リストを追加
- mouse, keyboard, cell phone, laptop, tv, remote, book, clock

**残課題**:
- `vase`, `potted plant`は廊下に設置されることがあるため、含めるか検討の余地あり

---

### 3.6 outdoor（屋外）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 屋外・外周 | |
| **version** | 1.0.0 | |
| **distance** | far | ✅ 適切 |
| **expected_objects** | human, vehicle, animal | ✅ 適切 |
| **excluded_objects** | 空 | ✅ 屋外なので適切 |
| **conf_override** | 0.5 | ✅ 誤検知抑制 |
| **interval** | 20秒 | ✅ 適切 |

**評価: ★★★★★ 優秀**

屋外環境では室内オブジェクトの誤検知リスクがないため、現状設定で適切。動物も検出対象に含めている点は、外周警備において有用。

**改善提案**: なし

---

### 3.7 night_vision（夜間）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 夜間・低照度 | |
| **version** | 1.0.0 | |
| **distance** | medium | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | 空 | ❌ 問題あり |
| **conf_override** | 0.3（低め） | ⚠️ 誤検知増リスク |
| **par_threshold** | 0.5 | ✅ 低照度対応 |
| **interval** | 15秒 | ✅ 適切 |

**評価: ★★★☆☆ 要改善**

**問題点**:
- 夜間モードは屋内・屋外両方で使用される
- 低conf閾値（0.3）+ 空の除外リストは誤検知リスク大
- IR照明下でオフィス用品が検出される可能性

**改善提案**:
```rust
excluded_objects: vec![
    "mouse", "keyboard", "laptop", "cell phone",
    "tv", "remote", "book",
]
conf_override: Some(0.35),  // 0.3→0.35に微調整
```

---

### 3.8 crowd（群衆）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 群衆・混雑検知 | |
| **version** | 1.0.0 | |
| **distance** | far | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | 空 | ⚠️ 検討余地あり |
| **conf_override** | 0.25（最低） | ⚠️ 誤検知リスク大 |
| **nms_threshold** | 0.4 | ✅ 多人数対応 |
| **return_bboxes** | false | ✅ パフォーマンス配慮 |
| **interval** | 30秒 | ✅ 適切 |

**評価: ★★★☆☆ 要改善**

**問題点**:
- 極低conf閾値（0.25）は誤検知を大量に発生させる
- イベント会場等で什器が誤検出される可能性

**改善提案**:
```rust
conf_override: Some(0.30),  // 0.25→0.30に引き上げ
excluded_objects: vec![
    "mouse", "keyboard", "laptop",  // 最低限追加
]
```

---

### 3.9 retail（小売店）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 店舗・小売 | |
| **version** | 1.0.0 | |
| **distance** | near | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | vehicle | ⚠️ 不十分 |
| **conf_override** | 0.35 | ✅ 適切 |
| **par_threshold** | 0.4 | ✅ 顧客属性取得 |
| **interval** | 10秒 | ✅ 適切 |

**評価: ★★★☆☆ 要改善**

**問題点**:
- レジ周りのマウス・キーボードが検出される
- 店内商品（tv, cell phone等）は検出されるべきか？→顧客との区別が必要

**改善提案**:
```rust
excluded_objects: vec![
    "vehicle",
    "mouse", "keyboard",  // レジ周り対策
]
// 注: cell phone, laptopは顧客所持物として検出許可
```

---

### 3.10 office（オフィス）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | オフィス・会議室 | |
| **version** | 1.1.0 | ✅ 修正済 |
| **distance** | medium | ✅ 適切 |
| **expected_objects** | human | ✅ 適切 |
| **excluded_objects** | 9項目（修正後） | ✅ 改善済 |
| **conf_override** | None | ✅ 適切 |
| **enable_frame_diff** | false | ⚠️ 検討余地あり |
| **interval** | 30秒 | ✅ 適切 |

**評価: ★★★★☆ 良好（修正後）**

**修正内容（v1.1.0）**:
- vehicle, animalのみだったexcluded_objectsにオフィス用品を追加
- mouse, keyboard, laptop, cell phone, tv, remote, book

**検討事項**:
- `enable_frame_diff: false`の妥当性
  - オフィスでは動きが少ないという想定だが、人の出入りはある
  - 会議室用途ならfalseで良いが、オープンオフィスではtrueが適切な場合も

---

### 3.11 warehouse（倉庫）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | 倉庫・物流施設 | |
| **version** | 1.0.0 | |
| **distance** | far | ✅ 適切 |
| **expected_objects** | human, vehicle | ✅ 適切（フォークリフト等） |
| **excluded_objects** | 空 | ⚠️ 検討余地あり |
| **conf_override** | 0.5 | ✅ 遠距離対応 |
| **interval** | 20秒 | ✅ 適切 |

**評価: ★★★★☆ 良好**

**分析**:
- 倉庫内では通常オフィス用品は少ない
- ただし、管理事務所エリアが映り込む場合は誤検知リスクあり

**改善提案**:
- 大規模倉庫では問題なし
- 小規模倉庫（事務所併設）向けに`warehouse_office`サブプリセットを検討

---

### 3.12 custom（カスタム）

| 項目 | 設定値 | 評価 |
|------|--------|------|
| **用途** | ユーザー個別設定 | |
| **version** | 1.0.0 | |
| **distance** | None | ✅ ユーザー設定 |
| **expected_objects** | 空 | ✅ ユーザー設定 |
| **excluded_objects** | 空 | ✅ ユーザー設定 |
| **conf_override** | None | ✅ ユーザー設定 |
| **interval** | 15秒 | ✅ デフォルト |

**評価: ★★★★★ 優秀**

空のテンプレートとして適切。ユーザーが自由に設定可能。

**改善提案**: なし

---

## 4. 設計上の課題と改善提案

### 4.1 課題1: 除外リストの不統一

**現状**:
| プリセット | excluded_objects数 | 問題 |
|-----------|-------------------|------|
| balanced | 14 | 最も充実 |
| corridor | 8（修正後） | 改善済 |
| office | 9（修正後） | 改善済 |
| その他 | 0〜2 | 室内使用時にリスク |

**改善提案**: 共通除外リストの定数化

```rust
/// 室内共通除外リスト
const COMMON_INDOOR_EXCLUDED: &[&str] = &[
    "mouse", "keyboard", "cell phone", "laptop",
    "tv", "remote", "book", "clock"
];

/// オフィス追加除外（室内共通 + 装飾品）
const OFFICE_ADDITIONAL_EXCLUDED: &[&str] = &[
    "vase", "potted plant", "teddy bear"
];
```

### 4.2 課題2: 閾値設定の整合性

**現状の閾値分布**:

| conf_override | プリセット |
|---------------|-----------|
| 0.25 | crowd |
| 0.30 | person_priority, night_vision |
| 0.35 | entrance, retail |
| 0.45 | parking |
| 0.50 | outdoor, warehouse |
| None | balanced, corridor, office, custom |

**分析**:
- 低閾値（0.25-0.30）は検出漏れを防ぐが誤検知増
- 高閾値（0.45-0.50）は誤検知を防ぐが検出漏れ増
- `None`はis21のデフォルト値に依存

**改善提案**:
- `crowd`の0.25は極端に低い → 0.30に引き上げ推奨
- `night_vision`は低照度ノイズ対策で低めだが、室内除外リストがないと誤検知増

### 4.3 課題3: プリセット間の役割重複

**重複が見られるペア**:
- `entrance` vs `retail`: 両方とも近距離・人物優先
- `corridor` vs `office`: 両方とも室内・人物優先

**改善提案**:
- 差別化ポイントを明確化
- entranceは「来訪者属性取得」、retailは「顧客行動分析」
- corridorは「通過検知」、officeは「在席検知」

---

## 5. 推奨ユースケースマッピング

### 5.1 設置場所別推奨プリセット

| 設置場所 | 推奨プリセット | 理由 |
|----------|---------------|------|
| オフィス執務室 | balanced または office | オフィス用品除外 |
| 会議室 | office | 動き少・長間隔 |
| 廊下・通路 | corridor | 室内除外済 |
| エントランス | entrance | 来訪者詳細取得 |
| 屋外駐車場 | parking | 車両+人物検出 |
| 屋内駐車場 | balanced | 管理室映り込み対策 |
| 店舗売場 | retail | 顧客属性取得 |
| 倉庫（広域） | warehouse | 遠距離・フォークリフト |
| 倉庫（事務所併設） | balanced | オフィス用品除外 |
| 屋外外周 | outdoor | 動物含む全検出 |
| 夜間専用 | night_vision | 低照度最適化 |
| イベント会場 | crowd | 多人数カウント |
| 特殊要件 | custom | 個別設定 |

### 5.2 問題発生時のフォールバック

```
問題発生 → balanced に変更 → 改善確認 → 最適プリセット再選定
```

`balanced`は最も除外リストが充実しており、問題発生時の安全なフォールバック先。

---

## 6. 今後の改善ロードマップ

### 6.1 短期（〜1週間）

| タスク | 優先度 | 状態 |
|--------|--------|------|
| サジェストロジック修正 | 高 | ✅ 完了 |
| corridor除外リスト追加 | 高 | ✅ 完了 |
| office除外リスト追加 | 高 | ✅ 完了 |
| person_priority除外リスト追加 | 中 | 未着手 |
| entrance除外リスト追加 | 中 | 未着手 |

### 6.2 中期（〜1ヶ月）

| タスク | 優先度 |
|--------|--------|
| 共通除外リスト定数の導入 | 中 |
| night_vision除外リスト追加 | 中 |
| crowd閾値見直し（0.25→0.30） | 中 |
| retail除外リスト追加 | 低 |

### 6.3 長期（〜3ヶ月）

| タスク | 優先度 |
|--------|--------|
| autoAttunement機能実装 | 高 |
| プリセットバリエーション追加（parking_indoor等） | 低 |
| ユーザーカスタムプリセット保存機能 | 低 |

---

## 7. 結論

### 7.1 評価サマリー

| プリセット | 総合評価 | 主な課題 |
|-----------|----------|----------|
| balanced | ★★★★★ | なし |
| person_priority | ★★★☆☆ | 室内除外不足 |
| parking | ★★★★☆ | 屋内駐車場未対応 |
| entrance | ★★★☆☆ | 室内除外不足 |
| corridor | ★★★★☆ | v1.1.0で改善済 |
| outdoor | ★★★★★ | なし |
| night_vision | ★★★☆☆ | 室内除外不足 |
| crowd | ★★★☆☆ | 閾値が極端に低い |
| retail | ★★★☆☆ | レジ周り対策不足 |
| office | ★★★★☆ | v1.1.0で改善済 |
| warehouse | ★★★★☆ | 事務所併設未対応 |
| custom | ★★★★★ | なし |

### 7.2 推奨アクション

1. **即時実施**: person_priority, entrance, night_vision への室内除外リスト追加
2. **短期実施**: 共通除外リスト定数の導入でコード重複解消
3. **中期実施**: autoAttunement機能によるカメラ別自動最適化

---

**報告書終了**
