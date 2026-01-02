# is21 (camimageEdge AI) ハードウェア仕様書

取得日: 2025-12-30
取得元: 192.168.3.116 (is21本体)

---

## ボード情報

| 項目 | 値 |
|------|-----|
| モデル | **Orange Pi 5 Plus** |
| SoC | **Rockchip RK3588** |
| ボードファミリ | rockchip-rk3588 |
| アーキテクチャ | ARM64 (aarch64) |

---

## CPU

| 項目 | 値 |
|------|-----|
| 総コア数 | **8コア** |
| big.LITTLE構成 | 4x Cortex-A76 + 4x Cortex-A55 |
| A76 最大周波数 | 2.256 GHz |
| A55 最大周波数 | 1.8 GHz |
| 最小周波数 | 408 MHz |

### CPUコア詳細
```
processor 0-3: Cortex-A55 (CPU part 0xd05)
processor 4-7: Cortex-A76 (CPU part 0xd0b)
```

---

## NPU (Neural Processing Unit)

| 項目 | 値 |
|------|-----|
| NPU | **RK3588 内蔵 RKNPU** |
| 理論性能 | **6 TOPS (INT8)** |
| コア数 | **3コア** |
| 現在周波数 | 1.0 GHz |
| 利用可能周波数 | 300/400/500/600/700/800/900/1000 MHz |
| ガバナー | rknpu_ondemand |
| ドライババージョン | 0.9.8 |

### NPU性能仕様 (RK3588公式)
- **INT8**: 6 TOPS
- **INT16**: 3 TOPS
- **FP16**: 1.5 TFLOPS

---

## メモリ

| 項目 | 値 |
|------|-----|
| 総容量 | **8 GB** (7.7 GiB) |
| 使用中 | 496 MiB |
| 空き | 3.9 GiB |
| 利用可能 | 7.3 GiB |
| Swap | 3.9 GiB (zram) |

---

## ストレージ

| デバイス | 容量 | タイプ | 用途 |
|----------|------|--------|------|
| mmcblk1 | 57.6 GB | eMMC | システム (/) |
| nvme0n1 | 931.5 GB | NVMe SSD | データ (未マウント) |

### NVMe詳細
- モデル: **Crucial CT1000P3PSSD8** (1TB)
- 現在未使用 (将来の映像/モデル格納用)

---

## GPU

| 項目 | 値 |
|------|-----|
| GPU | Mali-G610 MP4 |
| 現在周波数 | 300 MHz |

---

## ネットワーク

| インターフェース | 状態 | IP |
|-----------------|------|-----|
| enP4p65s0 | UP | 192.168.3.116/24 |
| enP3p49s0 | DOWN | - |

- **デュアル2.5GbE** 搭載（1ポート使用中）

---

## OS・ソフトウェア

| 項目 | 値 |
|------|-----|
| OS | **Armbian 25.11.2 noble** |
| ベース | Ubuntu 24.04 LTS |
| カーネル | 6.1.115-vendor-rk35xx |
| Python | 3.12.3 |
| RKNN Toolkit Lite | **2.3.2** |

### 主要Pythonパッケージ
| パッケージ | バージョン |
|-----------|-----------|
| rknn-toolkit-lite2 | 2.3.2 |
| numpy | 2.2.6 |
| opencv-python | 4.12.0.88 |
| fastapi | 0.128.0 |
| uvicorn | 0.39.0 |

---

## 現在のモデル

| ファイル | サイズ | 用途 |
|----------|--------|------|
| yolov5s-640-640.rknn | 8.4 MB | 物体検出（現行） |
| resnet18_for_rk3588.rknn | 12 MB | テスト用 |
| yolov8n.onnx | 12.9 MB | 変換前ONNX |

---

## 現在の推論性能

| 指標 | 値 |
|------|-----|
| 平均NPU処理時間 | **45.9 ms** |
| 推論成功率 | 100% |
| 総推論回数 | 6 |

---

## 温度

| センサー | 温度 |
|----------|------|
| CPU | 38-39°C |
| NPU | 37-38°C |

---

## RK3588 NPU 処理能力の目安

| モデル | サイズ | 推定処理時間 |
|--------|--------|-------------|
| YOLOv5s (640x640) | 8MB | ~40-50ms |
| YOLOv5m (640x640) | 25MB | ~80-100ms |
| YOLOv8n (640x640) | 6MB | ~30-40ms |
| YOLOv8s (640x640) | 22MB | ~60-80ms |
| ResNet50 (224x224) | 25MB | ~15-20ms |
| MobileNetV3 (224x224) | 5MB | ~5-10ms |

---

## 追加モデル実行可能性

RK3588のNPU (6 TOPS INT8) で実行可能と想定されるモデル:

| カテゴリ | モデル例 | 実行可能性 |
|----------|---------|-----------|
| 物体検出 | YOLOv5s/m, YOLOv8n/s | ○ |
| 人物属性 | OSNet, MGN (ReID) | ○ |
| 姿勢推定 | MoveNet, PoseNet | ○ |
| 顔検出 | RetinaFace, SCRFD | ○ |
| 色分類 | MobileNet + FC | ○ |
| 行動認識 | SlowFast (軽量版) | △ |
| VLM | LLaVA, BLIP-2 | × (NPUでは困難) |

**結論**: 属性認識モデルの追加はNPU性能的に可能

---

## 接続情報

| 項目 | 値 |
|------|-----|
| IP | 192.168.3.116 |
| SSH | mijeosadmin / mijeos12345@ |
| API | http://192.168.3.116:9000 |
| ホスト名 | araneais21camimageEdgeAI |

---

作成: Claude Code
