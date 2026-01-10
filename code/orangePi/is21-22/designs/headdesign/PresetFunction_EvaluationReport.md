# プリセット機能 実装確認評価報告書

**Issue**: #107 プリセットグラフィカルUI・過剰検出判定
**作成日**: 2026-01-08
**評価者**: Claude Code
**対象システム**: is22 RTSPカメラ総合管理サーバー

---

## 1. エグゼクティブサマリー

### 評価結果: ✅ 修正完了（2026-01-08）

| 項目 | 評価 | 備考 |
|------|------|------|
| プリセット設定の整合性 | ✅ 修正済 | corridor/officeにexcluded_objects追加 |
| サジェストロジック | ✅ 修正済 | 原因ベースの推奨ロジックに改善 |
| UI/UX | ✅ 良好 | PresetSelector動作確認済 |
| バランスチャート | ✅ 良好 | アニメーション正常 |

---

## 2. プリセット設定の詳細評価

### 2.1 全12プリセット比較表

| ID | 名称 | excluded_objects | expected_objects | conf_override | 評価 |
|----|------|------------------|------------------|---------------|------|
| `person_priority` | 人物優先 | vehicle, animal | human | 0.3 | ⚠️ オフィス用品未除外 |
| `balanced` | バランス | **14項目** ※1 | human | None | ✅ 最も安全 |
| `parking` | 駐車場 | **空** | human, vehicle | 0.45 | ⚠️ mouse誤検知リスク |
| `entrance` | エントランス | vehicle | human | 0.35 | ⚠️ オフィス用品未除外 |
| `corridor` | 廊下 | **空** | human | None | ❌ **危険** |
| `outdoor` | 屋外 | **空** | human, vehicle, animal | 0.5 | ⚠️ 屋外なので許容 |
| `night_vision` | 夜間 | **空** | human | 0.3 | ⚠️ mouse誤検知リスク |
| `crowd` | 群衆 | **空** | human | 0.25 | ⚠️ 低閾値で誤検知増 |
| `retail` | 小売店 | vehicle | human | 0.35 | ⚠️ オフィス用品未除外 |
| `office` | オフィス | vehicle, animal | human | None | ⚠️ **オフィスなのにmouse未除外** |
| `warehouse` | 倉庫 | **空** | human, vehicle | 0.5 | ⚠️ mouse誤検知リスク |
| `custom` | カスタム | **空** | 空 | None | N/A（ユーザー設定） |

**※1 `balanced`のexcluded_objects（14項目）:**
```
mouse, keyboard, cell phone, laptop, tv, book, clock, vase,
potted plant, teddy bear, remote, toothbrush, hair drier, scissors
```

### 2.2 excluded_objects設計の問題点

#### 問題1: `corridor`（廊下）プリセットの設計不備

```rust
// 現状: excluded_objectsが空
pub fn corridor() -> Self {
    Self {
        excluded_objects: vec![],  // ← 問題
        // ...
    }
}
```

**影響**: 廊下にカメラを設置した場合でも、近くにあるデスク上のマウス・キーボード等が検出され続ける。

#### 問題2: `office`（オフィス）プリセットの矛盾

```rust
// 現状: vehicleとanimalのみ除外
pub fn office() -> Self {
    Self {
        excluded_objects: vec!["vehicle".to_string(), "animal".to_string()],
        // ...
    }
}
```

**矛盾**: オフィス環境なのに、オフィスに必ず存在する`mouse`, `keyboard`, `laptop`等が除外されていない。

#### 問題3: 用途別除外リストの不整合

| 環境タイプ | 必要な除外対象 | 現状 |
|-----------|---------------|------|
| 室内オフィス系 | mouse, keyboard, laptop, cell phone, tv | `balanced`のみ対応 |
| 屋外駐車場系 | 植物系は許容 | 適切 |
| 商業施設系 | 商品は除外不可だが什器は除外したい | 未対応 |

---

## 3. サジェストロジックの重大バグ

### 3.1 現行ロジック（EventLogPane.tsx:674-686）

```typescript
// 問題のあるロジック
let recommendedPreset = 'balanced'

if (hasHighFrequency) {
    recommendedPreset = 'corridor'  // ← 問題！
} else if (hasTagFixation) {
    recommendedPreset = 'person_priority'  // ← 問題！
}
```

### 3.2 バグの詳細

#### バグ1: 高頻度検出時に「廊下」を推奨

| 状況 | 推奨プリセット | 結果 |
|------|---------------|------|
| mouseが大量検出（高頻度） | `corridor`（廊下） | ❌ **悪化** |

**理由**: `corridor`はexcluded_objectsが空なので、mouseのフィルタリングが効かない。

**実際に発生した事象**:
1. Tam-1F-Frontカメラで机上のコンピューターマウスを検出
2. 元々「バランス」プリセット → mouseが除外されていた
3. AIアシスタントが「廊下」への変更を推奨
4. ユーザーが「はい」をクリック
5. mouseがフィルタリングされなくなり、**Unknown乱発**発生

#### バグ2: タグ固定化時に「人物優先」を推奨

| 状況 | 推奨プリセット | 結果 |
|------|---------------|------|
| 特定タグが80%超過（タグ固定化） | `person_priority`（人物優先） | △ 不適切な場合あり |

**理由**: `person_priority`はvehicle, animalのみ除外で、オフィス用品は除外されない。

### 3.3 バグの根本原因

サジェストロジックが**過剰検出の原因**を考慮していない：

1. **高頻度検出の原因パターン**:
   - 静止物の繰り返し検出（mouse等） → `balanced`推奨が正解
   - 人の往来が多い → `crowd`推奨が正解
   - ポーリング間隔が短すぎる → 設定変更推奨が正解

2. **現行ロジックの問題**:
   - 原因を区別せず一律「廊下」を推奨
   - excluded_objectsの内容を考慮していない

---

## 4. YOLOラベル「mouse」の説明

### 4.1 YOLOv8/YOLOv5のCOCOラベル

| ラベル | 意味 | カテゴリ |
|--------|------|----------|
| `mouse` | コンピューターマウス | オフィス用品 |
| `rat` | ネズミ（動物） | 動物 ※COCOには存在しない |

**重要**: YOLOのCOCOデータセットには**動物のネズミ（rat）は含まれていません**。`mouse`は純粋にオフィス用品としてのコンピューターマウスを指します。

### 4.2 監視カメラ環境での「mouse」検出

| 設置場所 | mouseの検出 | 推奨対応 |
|----------|-------------|----------|
| オフィス | 頻繁（デスク上） | 除外必須 |
| エントランス | まれ（受付デスク） | 除外推奨 |
| 廊下 | 可能性あり（近くの部屋） | 除外推奨 |
| 駐車場 | ほぼなし | 除外不要 |
| 屋外 | なし | 除外不要 |

---

## 5. 改善提案

### 5.1 プリセット設定の修正

#### 提案1: 共通除外リストの導入

```rust
// 新規: 共通除外リスト（室内向け）
const COMMON_INDOOR_EXCLUDED: &[&str] = &[
    "mouse", "keyboard", "laptop", "cell phone",
    "tv", "remote", "book", "clock"
];

// 各プリセットで利用
pub fn corridor() -> Self {
    Self {
        excluded_objects: COMMON_INDOOR_EXCLUDED.iter().map(|s| s.to_string()).collect(),
        // ...
    }
}
```

#### 提案2: プリセット別修正内容

| プリセット | 修正内容 |
|-----------|----------|
| `corridor` | COMMON_INDOOR_EXCLUDED追加 |
| `office` | mouse, keyboard, laptop, cell phone追加 |
| `entrance` | mouse, keyboard, cell phone追加 |
| `retail` | mouse, keyboard追加（レジ周り対応） |
| `night_vision` | COMMON_INDOOR_EXCLUDED追加 |

### 5.2 サジェストロジックの修正

#### 提案: 原因ベースの推奨ロジック

```typescript
// 改善版ロジック
function getRecommendedPreset(issues: OverdetectionIssue[], currentPreset: string): string {
    // Unknown乱発 → excluded_objectsが多いプリセットを推奨
    const hasUnknownFlood = issues.some(i => i.type === 'unknown_flood')
    if (hasUnknownFlood) {
        return 'balanced'  // 14項目除外で最も安全
    }

    // 高頻度検出 → 原因を分析
    const hasHighFrequency = issues.some(i => i.type === 'high_frequency')
    if (hasHighFrequency) {
        // 静止物が原因なら除外リストが多いプリセット
        const hasStaticObject = issues.some(i => i.type === 'static_object')
        if (hasStaticObject) {
            return 'balanced'
        }
        // 動的な人物が多い場合は群衆モード
        return 'crowd'
    }

    // タグ固定化 → 問題のタグを分析
    const tagFixation = issues.find(i => i.type === 'tag_fixation')
    if (tagFixation) {
        // オフィス用品タグなら除外リストが多いプリセット
        if (['mouse', 'keyboard', 'laptop'].includes(tagFixation.tag || '')) {
            return 'balanced'
        }
        // 人物系タグなら人物優先
        return 'person_priority'
    }

    return currentPreset  // 変更不要
}
```

### 5.3 UIの改善提案

#### 提案: サジェスト理由の明示

現在:
```
Tam-1F-Frontの検出の調整が必要そうです。
現在「バランス」→「廊下」をお試しください。変更しますか？
```

改善後:
```
Tam-1F-Frontの検出の調整が必要そうです。
【原因】静止物（mouse）の繰り返し検出
【推奨】「バランス」のままで問題ありません（mouseは除外済み）
または、検出感度を下げますか？
```

---

## 6. 実装優先度

| 優先度 | タスク | 工数目安 | 影響範囲 | 状態 |
|--------|--------|----------|----------|------|
| 🔴 高 | サジェストロジックのバグ修正 | 2h | EventLogPane.tsx | ✅ 完了 |
| 🔴 高 | `corridor`プリセットの修正 | 0.5h | preset_loader/mod.rs | ✅ 完了 |
| 🟡 中 | `office`プリセットの修正 | 0.5h | preset_loader/mod.rs | ✅ 完了 |
| 🟡 中 | 共通除外リストの導入 | 2h | preset_loader/mod.rs | ⏳ 未着手 |
| 🟢 低 | サジェスト理由の明示UI | 4h | EventLogPane.tsx | ✅ 完了 |

---

## 7. テストケース

### 7.1 修正後の検証項目

| # | テスト内容 | 期待結果 |
|---|-----------|----------|
| 1 | `corridor`プリセット適用後、mouseが検出されないこと | PASS |
| 2 | Unknown乱発時に`balanced`が推奨されること | PASS |
| 3 | 高頻度+静止物検出時に`balanced`が推奨されること | PASS |
| 4 | `office`プリセット適用後、mouseが検出されないこと | PASS |

### 7.2 回帰テスト

| # | テスト内容 | 期待結果 |
|---|-----------|----------|
| 1 | 人物検知が正常に動作すること | PASS |
| 2 | プリセット変更時のアニメーションが動作すること | PASS |
| 3 | 過剰検出警告が正しく表示されること | PASS |

---

## 8. 結論

### 8.1 発見された問題

1. **設計上の問題**: プリセット間でexcluded_objectsの設計方針が不統一
2. **実装バグ**: サジェストロジックが逆効果のプリセットを推奨
3. **ラベル理解不足**: `mouse`がオフィス用品であることの認識不足

### 8.2 推奨アクション

1. **即時対応**: サジェストロジックのバグ修正
2. **短期対応**: `corridor`, `office`プリセットの修正
3. **中期対応**: 共通除外リストの導入とテスト
4. **長期対応**: autoAttunement機能での自動調整（TODO参照）

---

## 付録A: 現行プリセット設定の完全リスト

```rust
// preset_loader/mod.rs から抜粋

// balanced (推奨デフォルト)
excluded_objects: vec![
    "mouse", "keyboard", "cell phone", "laptop", "tv",
    "book", "clock", "vase", "potted plant", "teddy bear",
    "remote", "toothbrush", "hair drier", "scissors"
]

// person_priority
excluded_objects: vec!["vehicle", "animal"]

// parking
excluded_objects: vec![]

// entrance
excluded_objects: vec!["vehicle"]

// corridor ← v1.1.0で修正済
excluded_objects: vec![
    "mouse", "keyboard", "cell phone", "laptop",
    "tv", "remote", "book", "clock"
]

// outdoor
excluded_objects: vec![]

// night_vision
excluded_objects: vec![]

// crowd
excluded_objects: vec![]

// retail
excluded_objects: vec!["vehicle"]

// office ← v1.1.0で修正済
excluded_objects: vec![
    "vehicle", "animal", "mouse", "keyboard",
    "laptop", "cell phone", "tv", "remote", "book"
]

// warehouse
excluded_objects: vec![]

// custom
excluded_objects: vec![]
```

---

**報告書終了**
