# is21 推論テスト結果報告

テスト日時: 2025-12-30 06:50 JST
テスト環境: Orange Pi 5 Plus (RK3588 NPU)
モデル: yolov5s-640-640.rknn

---

## テスト結果サマリー

### 結果比較表

| 画像 | 正解 | is21結果 | 判定 |
|------|------|----------|------|
| camera-night-installation2.jpg | person:1, human | person:7(?), unknown | **NG** |
| IMG_1F097BD1F07C-1.jpg | person:2, human | person:9(?), unknown | **NG** |
| security-camera-image03.webp | person:1, human | animal(?), count:12 | **NG** |

### 処理性能（これは良好）

| 指標 | 値 | 目標 | 判定 |
|------|-----|------|------|
| 平均処理時間 | 93.37ms | <300ms | **OK** |
| 成功率 | 100% | >99% | **OK** |
| NPU利用 | 有効 | - | **OK** |

---

## 詳細結果

### 画像1: camera-night-installation2.jpg

**正解**:
- person: 1名（黒服・目出し帽の不審者）
- primary_event: human
- severity: 高

**is21結果**:
```json
{
  "detected": true,
  "primary_event": "unknown",
  "tags": ["count.multiple"],
  "confidence": 0.524,
  "count_hint": 7
}
```

**問題点**:
- [ ] person 1名なのに7名と誤検出
- [ ] primary_event が human ではなく unknown
- [ ] bbox座標が異常（0.001程度の極小値）
- [ ] 大量の誤検出（refrigerator, bicycle, bed, cup等）

---

### 画像2: IMG_1F097BD1F07C-1.jpg

**正解**:
- person: 2名（手前1名＋奥1名）
- primary_event: human

**is21結果**:
```json
{
  "detected": true,
  "primary_event": "unknown",
  "tags": ["count.multiple"],
  "confidence": 0.531,
  "count_hint": 9,
  "processing_ms": 87
}
```

**問題点**:
- [ ] person 2名なのに9名と誤検出
- [ ] primary_event が human ではなく unknown

---

### 画像3: security-camera-image03.webp

**正解**:
- person: 1名（白シャツ）
- primary_event: human

**is21結果**:
```json
{
  "detected": true,
  "primary_event": "animal",
  "tags": ["count.multiple"],
  "confidence": 0.528,
  "count_hint": 12,
  "processing_ms": 86
}
```

**問題点**:
- [ ] person 1名なのに12件と誤検出
- [ ] primary_event が human ではなく **animal**（完全に誤り）
- [ ] 人がいるのに動物と判定

---

## 根本原因分析

### 1. bbox座標の異常（最重要）

**症状**: 全bboxのx1/y1/x2/y2が0.001〜0.002程度の極小値

**原因推定**:
- YOLOv5後処理（`postprocess_yolov5`）の座標変換ロジックに問題
- RKNNモデル出力のフォーマットがPyTorch版と異なる可能性
- anchor/stride計算が未実装または誤り

**影響**:
- bboxが画像の極めて小さい領域を指すため、実質的に検出として無意味
- NMSが正しく機能せず、大量の誤検出が残る

### 2. 過剰検出（False Positive多発）

**症状**: 1-2名の人物に対して7-12件の検出

**原因推定**:
- CONF_THRESHOLD（0.25）が低すぎる
- NMS_THRESHOLD（0.45）の調整が必要
- bbox座標異常のためNMSが機能していない

### 3. primary_event誤判定

**症状**: personを検出しているはずなのにunknown/animalと判定

**原因推定**:
- 誤検出された非人物オブジェクトが最高confidence
- EVENT_MAPに含まれないクラスが最高スコア

---

## 調整必要項目

### 優先度: Critical（P0）

#### TUNE-001: YOLOv5後処理の座標変換修正

**問題**: RKNNモデル出力の座標が正しく変換されていない

**調査項目**:
1. RKNNLite出力のshapeとフォーマットを確認
2. PyTorch YOLOv5の出力フォーマットと比較
3. anchor計算の有無を確認

**対応案**:
```python
# 現在の後処理を確認・修正
def postprocess_yolov5(outputs, img_shape):
    # RKNNの出力フォーマットを確認
    for i, out in enumerate(outputs):
        print(f"Output {i}: shape={out.shape}, dtype={out.dtype}")
        print(f"  min={out.min()}, max={out.max()}")
```

**担当**: 推論パイプライン
**期限**: 最優先

---

### 優先度: High（P1）

#### TUNE-002: 信頼度閾値の調整

**現在値**: CONF_THRESHOLD = 0.25

**調整方針**:
- 0.25 → 0.5 に引き上げて誤検出削減
- 段階的にテスト: 0.3, 0.4, 0.5, 0.6

**検証方法**:
```bash
# 閾値別テスト
for threshold in 0.3 0.4 0.5 0.6; do
  curl -X POST ... -F "hints_json={\"conf_threshold\": $threshold}" ...
done
```

---

#### TUNE-003: NMS閾値の調整

**現在値**: NMS_THRESHOLD = 0.45

**調整方針**:
- IoU閾値を下げて重複削減: 0.45 → 0.3
- bbox座標修正後に再調整

---

### 優先度: Medium（P2）

#### TUNE-004: グレースケール/IR画像対応

**問題**: 夜間監視カメラのグレースケール画像で精度低下

**対応案**:
1. 入力時にグレースケール→RGB変換（3ch複製）
2. ヒストグラム正規化で コントラスト改善
3. 専用の夜間モデル検討

---

#### TUNE-005: 監視カメラ特化の前処理

**対応案**:
1. レターボックス処理の確認（アスペクト比維持）
2. 解像度別の最適化（640, 960, 1280）
3. 魚眼歪み補正（オプション）

---

### 優先度: Low（P3）

#### TUNE-006: モデル検証

**調査項目**:
1. RKNN変換時の精度劣化確認
2. INT8量子化の影響評価
3. 代替モデル検討（YOLOv8, YOLOv5m）

---

#### TUNE-007: 推論パラメータの動的設定

**機能追加**:
- API経由での閾値調整
- カメラ別プロファイル対応
- hints_json活用

---

## 次のアクション

### Phase 1: 座標変換修正（TUNE-001）

1. RKNNLite出力のデバッグログ追加
2. 出力フォーマットの解析
3. 座標変換ロジックの修正
4. テスト画像で再検証

### Phase 2: 閾値調整（TUNE-002, 003）

1. CONF_THRESHOLD を 0.5 に変更
2. テスト実行
3. 結果に応じて微調整

### Phase 3: 前処理改善（TUNE-004, 005）

1. グレースケール画像の前処理追加
2. コントラスト正規化
3. 再テスト

---

## 参考: 現在の後処理コード

```python
# main.py: postprocess_yolov5() - 要修正箇所
def postprocess_yolov5(outputs, img_shape):
    # RKNNの出力がanchordecodeされているかどうか不明
    # PyTorch版YOLOv5はgrid + anchor計算が必要
    for i, output in enumerate(outputs):
        output = output.reshape(-1, 85)  # ← この仮定が正しいか？
        # ...
```

---

## 結論

**現状**: 推論は動作するが、後処理に致命的な問題があり実用不可

**改善優先度**:
1. **TUNE-001（座標変換）**: 最優先、これが直らないと他の調整が無意味
2. **TUNE-002/003（閾値調整）**: 座標修正後に実施
3. **TUNE-004/005（前処理）**: 精度向上のため順次対応

**想定工数**: 2-3日（座標変換問題の複雑さによる）
