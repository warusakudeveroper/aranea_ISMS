# プリセット再設計 v2.0

**Issue**: #107 プリセット再設計
**作成日**: 2026-01-09
**設計者**: Claude Code
**対象システム**: is22 RTSPカメラ総合管理サーバー

---

## 1. 設計原則

### 1.1 The_golden_rules.md準拠

- **SSoT**: プリセット定義はpreset_loader/mod.rsに一元化
- **SOLID**: 単一責任（各プリセットは明確な用途を持つ）
- **MECE**: プリセット間で用途が重複せず、かつ主要ユースケースを網羅
- **アンアンビギュアス**: 各プリセットの目的と適用シーンを明確に定義

### 1.2 プリセット差別化の基本方針

**閾値（conf_override等）はプリセットの本質的差別化要因ではない**

理由:
- カメラ個別に閾値スライダーで調整可能（API実装済み、UI実装予定）
- 同じ用途でも環境により最適閾値は異なる

**本質的差別化要因**:
| 要因 | 説明 | スライダー調整 |
|------|------|----------------|
| `expected_objects` | 検出対象オブジェクト | ❌ 不可 |
| `excluded_objects` | 除外オブジェクト | ❌ 不可 |
| `enable_frame_diff` | フレーム差分解析 | ❌ 不可 |
| `output_schema` | 出力形式 | ❌ 不可 |
| `return_bboxes` | バウンディングボックス | ❌ 不可 |
| `conf_override` | 検出閾値 | ✅ スライダー可 |
| `par_threshold` | PAR閾値 | ✅ スライダー可 |
| `nms_threshold` | NMS閾値 | ✅ スライダー可 |

---

## 2. 既存プリセット重複分析

### 2.1 現行12プリセットの比較

| ID | expected | excluded数 | frame_diff | output_schema | 閾値設定 |
|----|----------|------------|------------|---------------|----------|
| balanced | human | 14 | ✓ | - | デフォルト |
| person_priority | human | 15 | ✓ | person_detailed | 低conf/低par |
| parking | human,vehicle | 0 | ✓ | parking | 高conf |
| entrance | human | 1 | ✓ | person_detailed | 中conf |
| corridor | human | 8 | ✓ | - | デフォルト |
| outdoor | human,vehicle,animal | 0 | ✓ | - | 高conf |
| night_vision | human | 0 | ✓ | - | 低conf |
| crowd | human | 0 | ✓ | crowd | 極低conf |
| retail | human | 1 | ✓ | person_detailed | 中conf |
| office | human | 9 | ❌ | - | デフォルト |
| warehouse | human,vehicle | 0 | ✓ | - | 高conf |
| custom | (空) | 0 | ✓ | - | デフォルト |

### 2.2 重複・曖昧なプリセットの特定

#### 重複ペア1: entrance ↔ retail
| 項目 | entrance | retail | 差異 |
|------|----------|--------|------|
| expected | human | human | 同一 |
| excluded | vehicle | vehicle | 同一 |
| frame_diff | ✓ | ✓ | 同一 |
| output_schema | person_detailed | person_detailed | 同一 |
| distance | near | near | 同一 |
| conf_override | 0.35 | 0.35 | 同一 |

**結論**: **完全重複** → 統合が必要

#### 重複ペア2: warehouse ↔ parking
| 項目 | warehouse | parking | 差異 |
|------|-----------|---------|------|
| expected | human,vehicle | human,vehicle | 同一 |
| excluded | 0 | 0 | 同一 |
| frame_diff | ✓ | ✓ | 同一 |
| output_schema | - | parking | **異なる** |
| distance | far | far | 同一 |
| conf_override | 0.5 | 0.45 | スライダー調整可 |

**結論**: output_schema以外同一 → **統合検討**

#### 曖昧プリセット: night_vision
- 「夜間」という時間帯の概念だが、実際には低conf閾値のみ
- 屋内夜間/屋外夜間で必要な設定は異なる
- excluded_objectsが空のため室内使用時にmouse誤検知

**結論**: 時間帯はプリセットではなく**モード**として扱うべき → **削除検討**

#### 類似プリセット: corridor ↔ balanced
| 項目 | corridor | balanced | 差異 |
|------|----------|----------|------|
| expected | human | human | 同一 |
| excluded | 8 | 14 | **異なる** |
| frame_diff | ✓ | ✓ | 同一 |
| distance | medium | medium | 同一 |
| location_type | corridor | (なし) | **異なる** |

**結論**: 除外リストの差は意図的 → **維持**（corridorは廊下特化）

---

## 3. 新プリセット設計

### 3.1 追加プリセット要件

| ID | 名称 | 用途 | expected_objects | 特徴 |
|----|------|------|------------------|------|
| hotel_corridor | ホテル廊下 | 宿泊施設廊下 | human, **door状態** | 人物+ドア開閉検知 |
| restaurant | 飲食店ホール | 飲食店 | human | 正確な人数カウント |
| security_zone | 警戒区域 | 重要エリア | **全オブジェクト** | あらゆる変化を厳密検知 |
| object_focus | 対物検知 | 物品監視 | **human以外** | 人を無視、静的物体変化 |

### 3.2 hotel_corridor（ホテル廊下）詳細設計

**目的**: 宿泊施設の廊下で人物とドアの状態を重点的に検知

```rust
pub fn hotel_corridor() -> Self {
    Self {
        id: "hotel_corridor".to_string(),
        name: "ホテル廊下".to_string(),
        description: "宿泊施設廊下向け（人物+ドア検知）".to_string(),
        version: "1.0.0".to_string(),
        location_type: Some("hotel_corridor".to_string()),
        distance: Some("medium".to_string()),
        // 人物とドアを重点検知
        expected_objects: vec!["human".to_string()],
        // ドアは検出対象として残す（excluded_objectsに入れない）
        // 室内用品は除外
        excluded_objects: vec![
            "mouse", "keyboard", "laptop", "cell phone",
            "tv", "remote", "book", "clock",
            "chair", "couch",  // 客室内什器
        ],
        enable_frame_diff: true,  // ドア開閉検知に必要
        return_bboxes: true,
        output_schema: Some("hotel".to_string()),
        conf_override: Some(0.35),
        nms_threshold: None,
        par_threshold: Some(0.45),  // 宿泊客属性を取得
        suggested_interval_sec: 10,
    }
}
```

**IS21側への要件**:
- `door`ラベルはYOLO COCOに含まれない
- **対応策**: frame_diff解析で「矩形領域の変化」を検出
- または、カスタムYOLOモデルでdoor検出を追加（将来課題）

### 3.3 restaurant（飲食店ホール）詳細設計

**目的**: 飲食店での正確な人数カウント

```rust
pub fn restaurant() -> Self {
    Self {
        id: "restaurant".to_string(),
        name: "飲食店ホール".to_string(),
        description: "飲食店向け（正確な人数カウント）".to_string(),
        version: "1.0.0".to_string(),
        location_type: Some("restaurant".to_string()),
        distance: Some("medium".to_string()),
        expected_objects: vec!["human".to_string()],
        // 飲食店固有: 食器類・調理器具は除外
        excluded_objects: vec![
            "mouse", "keyboard", "laptop", "cell phone",
            "tv", "remote", "book", "clock",
            "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl",
            "chair", "dining table",  // 什器
        ],
        enable_frame_diff: true,
        return_bboxes: false,  // 人数カウントのみ、bbox不要
        output_schema: Some("count".to_string()),
        conf_override: Some(0.35),
        nms_threshold: Some(0.5),  // 重複排除を厳密に（正確カウント）
        par_threshold: None,       // 属性不要、カウントのみ
        suggested_interval_sec: 15,
    }
}
```

### 3.4 security_zone（警戒区域）詳細設計

**目的**: 重要エリアであらゆる変化を厳密に検知

```rust
pub fn security_zone() -> Self {
    Self {
        id: "security_zone".to_string(),
        name: "警戒区域".to_string(),
        description: "重要エリア向け（あらゆる変化を厳密検知）".to_string(),
        version: "1.0.0".to_string(),
        location_type: Some("security".to_string()),
        distance: Some("medium".to_string()),
        // 全てのオブジェクトを検出対象
        expected_objects: vec![
            "human".to_string(),
            "vehicle".to_string(),
            "animal".to_string(),
        ],
        // 除外なし - 全て検知
        excluded_objects: vec![],
        enable_frame_diff: true,
        return_bboxes: true,
        output_schema: Some("security".to_string()),
        conf_override: Some(0.25),  // 低閾値で見逃し防止
        nms_threshold: Some(0.3),   // 重複許容
        par_threshold: Some(0.3),   // 詳細属性も取得
        suggested_interval_sec: 5,   // 高頻度ポーリング
    }
}
```

**注意**: 誤検知が多発する可能性があるが、セキュリティ優先のため許容

### 3.5 object_focus（対物検知）詳細設計

**目的**: 人を無視して静的オブジェクト・背景変化に注目

```rust
pub fn object_focus() -> Self {
    Self {
        id: "object_focus".to_string(),
        name: "対物検知".to_string(),
        description: "物品監視向け（人を無視、静的物体変化検知）".to_string(),
        version: "1.0.0".to_string(),
        location_type: Some("object".to_string()),
        distance: Some("near".to_string()),
        // 人以外のオブジェクトを検出
        expected_objects: vec![],  // 空 = 全検出（後でフィルタ）
        // 人を除外！（逆転の発想）
        excluded_objects: vec![
            "person".to_string(),
        ],
        enable_frame_diff: true,  // 背景変化検知に必須
        return_bboxes: true,
        output_schema: Some("object".to_string()),
        conf_override: Some(0.4),
        nms_threshold: None,
        par_threshold: None,  // 人物属性不要
        suggested_interval_sec: 30,  // 静的監視のため長間隔
    }
}
```

**ユースケース**:
- 倉庫の在庫変化監視
- 美術館の展示品監視
- 駐車場の車両有無確認（人は不要）

---

## 4. プリセット統廃合計画

### 4.1 廃止プリセット

| ID | 理由 | 代替 |
|----|------|------|
| retail | entranceと完全重複 | entrance使用 |
| night_vision | 閾値のみの差でモード的 | balanced + 低conf調整 |
| warehouse | parkingとほぼ同一 | parking使用 |

### 4.2 統合・名称変更

| 旧ID | 新ID | 変更内容 |
|------|------|----------|
| entrance | entrance | retailの機能を吸収、説明更新 |
| parking | parking | warehouseの機能を吸収、説明更新 |

### 4.3 改修プリセット

| ID | 改修内容 |
|----|----------|
| person_priority | v1.1.0で改修済 |
| corridor | v1.1.0で改修済 |
| office | v1.1.0で改修済 |
| balanced | 説明更新のみ |
| outdoor | 説明更新のみ |
| crowd | 閾値見直し（0.25→0.30） |
| custom | 維持 |

### 4.4 新規追加

| ID | 名称 |
|----|------|
| hotel_corridor | ホテル廊下 |
| restaurant | 飲食店ホール |
| security_zone | 警戒区域 |
| object_focus | 対物検知 |

---

## 5. 最終プリセット構成（v2.0）

### 5.1 プリセット一覧（12個 → 12個）

| # | ID | 名称 | 主目的 | expected | excluded数 |
|---|-----|------|--------|----------|------------|
| 1 | balanced | バランス | 汎用・デフォルト | human | 14 |
| 2 | person_priority | 人物優先 | 人物詳細属性取得 | human | 15 |
| 3 | entrance | エントランス | 来訪者検知 | human | 10 |
| 4 | corridor | 廊下 | 通路監視 | human | 8 |
| 5 | hotel_corridor | ホテル廊下 | 宿泊施設 | human | 10 |
| 6 | office | オフィス | 執務室・会議室 | human | 9 |
| 7 | restaurant | 飲食店ホール | 人数カウント | human | 15 |
| 8 | parking | 駐車場 | 車両+人物 | human,vehicle | 0 |
| 9 | outdoor | 屋外 | 外周警備 | human,vehicle,animal | 0 |
| 10 | crowd | 群衆 | 混雑検知 | human | 0 |
| 11 | security_zone | 警戒区域 | 厳密検知 | human,vehicle,animal | 0 |
| 12 | object_focus | 対物検知 | 物品監視 | (human除外) | 1 |
| 13 | custom | カスタム | ユーザー定義 | (空) | 0 |

**注**: 13個に増加（retail, night_vision, warehouse廃止、4個追加）

### 5.2 カテゴリ分類

```
室内系（Indoor）
├── balanced      # 汎用
├── person_priority  # 人物詳細
├── entrance      # 入口（retail吸収）
├── corridor      # 廊下
├── hotel_corridor   # ホテル廊下（新規）
├── office        # オフィス
└── restaurant    # 飲食店（新規）

屋外系（Outdoor）
├── parking       # 駐車場（warehouse吸収）
├── outdoor       # 外周
└── crowd         # 群衆

特殊系（Special）
├── security_zone # 警戒区域（新規）
├── object_focus  # 対物検知（新規）
└── custom        # カスタム
```

---

## 6. IS21側の改修要否

### 6.1 調査結果

IS21が受け付けるパラメータ:
- `hints_json`: CameraContextをJSON化したもの
- `expected_objects`, `excluded_objects`はヒントとして渡される
- IS21側でYOLO推論後にフィルタリング

### 6.2 重大なバグ発見: excluded_objectsのラベル不一致

**IS21のフィルタリングロジック** (main.py L654-659):
```python
label = bbox.get("label", "")  # YOLOラベル: "car", "person", "dog"等
if label in excluded:          # 直接文字列マッチ
    continue
```

**現行IS22のexcluded_objects**:
- `"vehicle"`, `"animal"` → カテゴリ名（**マッチしない！**）
- `"mouse"`, `"keyboard"` → YOLOラベル（正しくマッチ）

**結果**: `vehicle`, `animal`の除外は**機能していない**

### 6.3 YOLOv8 COCOラベル一覧（IS21 main.py L91-104）

```
person, bicycle, car, motorcycle, airplane, bus, train, truck, boat,
traffic light, fire hydrant, stop sign, parking meter, bench,
bird, cat, dog, horse, sheep, cow, elephant, bear, zebra, giraffe,
backpack, umbrella, handbag, tie, suitcase, frisbee, skis, snowboard,
sports ball, kite, baseball bat, baseball glove, skateboard, surfboard,
tennis racket, bottle, wine glass, cup, fork, knife, spoon, bowl,
banana, apple, sandwich, orange, broccoli, carrot, hot dog, pizza, donut, cake,
chair, couch, potted plant, bed, dining table, toilet, tv, laptop,
mouse, remote, keyboard, cell phone, microwave, oven, toaster, sink,
refrigerator, book, clock, vase, scissors, teddy bear, hair drier, toothbrush
```

### 6.4 修正方針

**IS22側で修正**（SSoT原則: IS22がYOLOラベルを管理）

| カテゴリ | 展開後YOLOラベル |
|---------|-----------------|
| vehicle | bicycle, car, motorcycle, airplane, bus, train, truck, boat |
| animal | bird, cat, dog, horse, sheep, cow, elephant, bear, zebra, giraffe |

### 6.5 改修要否判定（更新）

| 対象 | 改修 | 理由 |
|------|------|------|
| IS21 | ❌ 不要 | フィルタロジックは正常動作 |
| IS22 | ✅ 必要 | excluded_objectsをYOLOラベルに修正 |

### 6.6 object_focus確認結果

`excluded_objects: ["person"]`で正常にpersonが除外される（YOLOラベル直接マッチ）

---

## 7. テスト計画

### 7.1 単体テスト（Rust）

| # | テスト内容 | 期待結果 |
|---|-----------|----------|
| 1 | 全13プリセットが登録されること | presets.len() == 13 |
| 2 | 廃止プリセット(retail等)が存在しないこと | exists("retail") == false |
| 3 | 新規プリセットのto_camera_context()が正常動作 | Some(CameraContext) |
| 4 | balanced以外のget_or_default()がbalancedを返すこと | id == "balanced" |

### 7.2 統合テスト（API）

| # | エンドポイント | テスト内容 |
|---|---------------|-----------|
| 1 | GET /api/presets/balance | 13プリセット返却確認 |
| 2 | PUT /api/cameras/:id | 新プリセットID設定可能 |
| 3 | POST /api/analyze (mock) | hints_json正常送信 |

### 7.3 E2Eテスト（ブラウザ）

| # | シナリオ | 確認項目 |
|---|----------|----------|
| 1 | PresetSelectorで新プリセット選択 | ドロップダウン表示 |
| 2 | BalanceChartアニメーション | 正常描画 |
| 3 | カメラ設定保存 | DB反映確認 |
| 4 | 実際の検出動作 | IS21連携確認 |

---

## 8. タスクリスト（実装完了: 2026-01-09）

### Phase 0: バグ修正（最優先） ✅ 完了

- [x] 全プリセットのvehicle/animal除外をYOLOラベルに展開
  - vehicle → bicycle, car, motorcycle, airplane, bus, train, truck, boat
  - animal → bird, cat, dog, horse, sheep, cow, elephant, bear, zebra, giraffe
- [x] yolo_labels モジュール追加（VEHICLES, ANIMALS, OFFICE_ITEMS, INDOOR_MISC, FURNITURE, TABLEWARE）
- [x] combine_labels() ヘルパー関数追加

### Phase 1: 廃止・統合 ✅ 完了

- [x] retail, night_vision, warehouse プリセット廃止（スタブ関数で後方互換維持）
- [x] entrance説明更新（retail機能吸収）
- [x] parking説明更新（warehouse機能吸収）
- [x] crowd閾値修正（0.25→0.30）

### Phase 2: 新規追加 ✅ 完了

- [x] hotel_corridor プリセット実装
- [x] restaurant プリセット実装
- [x] security_zone プリセット実装
- [x] object_focus プリセット実装
- [x] preset_ids定数追加（HOTEL_CORRIDOR, RESTAURANT, SECURITY_ZONE, OBJECT_FOCUS）

### Phase 3: テスト・検証 ✅ 完了

- [x] 単体テスト更新（13プリセット対応）
- [x] YOLOラベル検証テスト追加
- [x] 廃止プリセットスタブテスト追加
- [ ] APIテスト実行（デプロイ後）
- [ ] E2Eテスト実行（デプロイ後）

### Phase 4: ドキュメント ✅ 完了

- [x] 設計ドキュメント最終化
- [x] フロントエンドPRESET_NAMES更新
- [x] overdetection_analyzer更新
- [ ] 評価報告書最終更新（デプロイ後）

### 実装ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `src/preset_loader/mod.rs` | yolo_labels, 新プリセット4種, 廃止スタブ3種 |
| `src/overdetection_analyzer/mod.rs` | get_all_preset_balances 13プリセット対応 |
| `frontend/src/components/EventLogPane.tsx` | PRESET_NAMES更新 |
| `frontend/src/components/PresetSelector/index.tsx` | 変更なし（APIから動的取得） |

---

## 9. MECE確認

### 9.1 ユースケースカバレッジ

| ユースケース | 対応プリセット | カバー |
|-------------|---------------|--------|
| 一般的な室内監視 | balanced | ✅ |
| 人物属性詳細取得 | person_priority | ✅ |
| 建物入口・店舗入口 | entrance | ✅ |
| 一般的な廊下・通路 | corridor | ✅ |
| ホテル・宿泊施設廊下 | hotel_corridor | ✅ |
| オフィス・会議室 | office | ✅ |
| 飲食店・カフェ | restaurant | ✅ |
| 駐車場・倉庫 | parking | ✅ |
| 屋外・外周 | outdoor | ✅ |
| イベント・混雑エリア | crowd | ✅ |
| 重要警戒エリア | security_zone | ✅ |
| 物品・在庫監視 | object_focus | ✅ |
| 特殊要件 | custom | ✅ |

### 9.2 重複排除確認

| プリセットA | プリセットB | 差異 | 重複判定 |
|------------|------------|------|----------|
| entrance | corridor | distance, excluded | ❌ 非重複 |
| corridor | hotel_corridor | output_schema, excluded | ❌ 非重複 |
| parking | outdoor | expected(animal) | ❌ 非重複 |
| crowd | restaurant | return_bboxes, nms | ❌ 非重複 |
| security_zone | outdoor | conf, interval | ❌ 非重複 |
| object_focus | (全て) | expected逆転 | ❌ 非重複 |

**結論**: MECE達成（重複なし、網羅性あり）

---

## 10. アンアンビギュアス確認

各プリセットの選択基準が明確か:

| 質問 | 回答プリセット | 曖昧性 |
|------|---------------|--------|
| 「オフィスに設置」 | office | ✅ 明確 |
| 「駐車場に設置」 | parking | ✅ 明確 |
| 「ホテルの廊下に設置」 | hotel_corridor | ✅ 明確 |
| 「レストランに設置」 | restaurant | ✅ 明確 |
| 「重要エリアで厳密監視したい」 | security_zone | ✅ 明確 |
| 「人は無視して物だけ見たい」 | object_focus | ✅ 明確 |
| 「とりあえず使いたい」 | balanced | ✅ 明確 |

**結論**: アンアンビギュアス達成

---

**設計ドキュメント終了**
