# is21推論テスト Ground Truth（6画像）

作成日: 2025-12-30
目的: is21 v1.1.0 推論精度検証の基準値

---

## テスト画像一覧

| # | ファイル名 | シーン | 難易度 |
|---|-----------|--------|--------|
| 1 | 25581835_m.jpg | ホテル客室（人なし） | 低 |
| 2 | camera-night-installation2.jpg | 室内監視（不審者） | 中 |
| 3 | IMG_1F097BD1F07C-1.jpg | 屋外監視（2名） | 高 |
| 4 | security-camera-image03.webp | 夜間屋外（1名） | 中 |
| 5 | security-night-02.jpg | ホテル廊下（人なし） | 低 |
| 6 | service-camera-02.jpg | 駐車場監視（車のみ） | 低 |

---

## 画像1: 25581835_m.jpg

### シーン説明
- **環境**: ホテル客室内
- **照明**: 室内照明（暖色系）
- **カメラ**: 通常カメラ（カラー）

### 正解ラベル

| オブジェクト | 数量 | 備考 |
|-------------|------|------|
| **person** | 0 | 人物なし |
| bed | 1 | ダブルベッド |
| chair | 1 | デスクチェア |

### 期待される推論結果

```json
{
  "detected": true/false,
  "primary_event": "none",
  "count_hint": 0,
  "person_count": 0
}
```

### 判定基準
- **PASS**: person検出なし、またはprimary_event != "human"
- **FAIL**: person検出あり かつ primary_event == "human"

---

## 画像2: camera-night-installation2.jpg

### シーン説明
- **環境**: リビングルーム（天井監視カメラ）
- **照明**: 夜間モード（グレースケール/IR）
- **カメラ情報**: CAM1, 02:57:38, REC表示

### 正解ラベル

| オブジェクト | 数量 | 位置 | 備考 |
|-------------|------|------|------|
| **person** | 1 | 画面中央左 | 黒服・目出し帽の不審者 |
| tv | 1 | 画面上部中央 | 壁掛けTV |
| couch | 1 | 画面右側 | グレーのL字ソファ |
| dining table | 1 | 画面中央 | 白いコーヒーテーブル |

### 期待される推論結果

```json
{
  "detected": true,
  "primary_event": "human",
  "count_hint": 1,
  "confidence": ">= 0.5"
}
```

### 判定基準
- **PASS**: primary_event == "human" かつ count_hint >= 1
- **FAIL**: primary_event != "human" または count_hint == 0

---

## 画像3: IMG_1F097BD1F07C-1.jpg

### シーン説明
- **環境**: 屋外（住宅街）
- **照明**: 夕暮れ～夜間（モノクロ/低照度）
- **カメラ情報**: 2022-05-05 19:07:49

### 正解ラベル

| オブジェクト | 数量 | 位置 | 備考 |
|-------------|------|------|------|
| **person** | 2 | 手前中央、奥右側 | 手前: ぼやけ（動きブレ）、奥: 小さい |

### 期待される推論結果

```json
{
  "detected": true,
  "primary_event": "human",
  "count_hint": 2,
  "confidence": ">= 0.4"
}
```

### 判定基準
- **PASS**: primary_event == "human" かつ count_hint >= 1
- **IDEAL**: count_hint == 2
- **FAIL**: primary_event != "human"

---

## 画像4: security-camera-image03.webp

### シーン説明
- **環境**: 屋外（建物入口付近）
- **照明**: 夜間（雨天、路面濡れ）
- **カメラ**: 広角/魚眼レンズ

### 正解ラベル

| オブジェクト | 数量 | 位置 | 備考 |
|-------------|------|------|------|
| **person** | 1 | 画面左側 | 白シャツ、歩行中 |

### 期待される推論結果

```json
{
  "detected": true,
  "primary_event": "human",
  "count_hint": 1,
  "confidence": ">= 0.5"
}
```

### 判定基準
- **PASS**: primary_event == "human" かつ count_hint >= 1
- **FAIL**: primary_event != "human"

---

## 画像5: security-night-02.jpg

### シーン説明
- **環境**: ホテル廊下
- **照明**: 薄暗い室内照明
- **カメラ**: 通常カメラ（カラー）

### 正解ラベル

| オブジェクト | 数量 | 備考 |
|-------------|------|------|
| **person** | 0 | 人物なし |

### 期待される推論結果

```json
{
  "detected": false,
  "primary_event": "none",
  "count_hint": 0
}
```

### 判定基準
- **PASS**: person検出なし、またはprimary_event != "human"
- **FAIL**: primary_event == "human"

---

## 画像6: service-camera-02.jpg

### シーン説明
- **環境**: 駐車場
- **照明**: 日中（曇り）
- **カメラ情報**: Camera 03, 07-14-2020 Tue 10:34:03

### 正解ラベル

| オブジェクト | 数量 | 備考 |
|-------------|------|------|
| **person** | 0 | 人物なし |
| car | 6-8 | 駐車中の車両多数 |
| truck | 1 | 白いバン（軽トラック） |

### 期待される推論結果

```json
{
  "detected": true,
  "primary_event": "vehicle",
  "count_hint": 0,
  "car_count": ">= 3"
}
```

### 判定基準
- **PASS**: primary_event == "vehicle" または person検出なし
- **FAIL**: primary_event == "human"

---

## テストサマリー

| # | ファイル名 | 期待primary_event | 期待person数 | 備考 |
|---|-----------|------------------|-------------|------|
| 1 | 25581835_m.jpg | none | 0 | 人なし確認 |
| 2 | camera-night-installation2.jpg | **human** | 1 | 不審者検出 |
| 3 | IMG_1F097BD1F07C-1.jpg | **human** | 2 | 複数人検出 |
| 4 | security-camera-image03.webp | **human** | 1 | 夜間人物検出 |
| 5 | security-night-02.jpg | none | 0 | 人なし確認 |
| 6 | service-camera-02.jpg | vehicle | 0 | 車両のみ |

### 合格基準

- **全6画像でPASS**: テスト合格
- **5画像以上でPASS**: 条件付き合格（調整要）
- **4画像以下でPASS**: テスト不合格（要修正）

---

## 検証環境

- **is21**: v1.1.0 (anchor decode修正版)
- **モデル**: yolov5s-640-640.rknn
- **CONF_THRESHOLD**: 0.5
- **NMS_THRESHOLD**: 0.45
- **NPU**: RK3588 (3コア)
