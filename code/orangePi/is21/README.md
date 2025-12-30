# IS21 camimageEdge AI

Orange Pi 5 Plus (RK3588 NPU) 向け Edge AI 推論サーバー

## 概要

| 項目 | 値 |
|-----|-----|
| デバイス型番 | ar-is21 |
| ハードウェア | Orange Pi 5 Plus |
| SoC | Rockchip RK3588 |
| NPU | 6 TOPS INT8 |
| ファームウェア | v1.8.0 |

## 機能

- **YOLOv5s** 物体検出（人物/車両/動物）
- **PAR (Person Attribute Recognition)** 人物属性認識
  - 性別/年齢/服装/所持品/姿勢
- **camera_context** コンテキストベース検知最適化
- **suspicious_score** 不審者スコア計算

## ディレクトリ構成

```
is21/
├── src/
│   ├── main.py              # メインサーバー
│   ├── par_inference.py     # PAR推論モジュール
│   └── aranea_common/       # 共通モジュール
├── config/
│   └── schema.json          # 推論スキーマ
├── systemd/
│   └── is21-infer.service   # systemdサービス定義
├── scripts/
│   ├── install.sh           # インストールスクリプト
│   └── update.sh            # 更新スクリプト
└── README.md
```

## インストール

### 前提条件

- Orange Pi 5 Plus
- Ubuntu 22.04 (Armbian)
- Python 3.10+
- rknn-toolkit2-lite

### 手順

```bash
# 1. リポジトリをクローン
git clone <repo_url>
cd aranea_ISMS/code/orangePi/is21

# 2. インストールスクリプト実行
sudo ./scripts/install.sh

# 3. モデルファイル配置
# 別途モデルファイルを /opt/is21/models/ に配置
#   - yolov5s-640-640.rknn
#   - par_resnet50_pa100k.rknn

# 4. サービス再起動
sudo systemctl restart is21-infer.service
```

### モデルファイル

以下のモデルファイルを `/opt/is21/models/` に配置:

| ファイル | サイズ | 用途 |
|---------|--------|------|
| yolov5s-640-640.rknn | ~28MB | 物体検出 |
| par_resnet50_pa100k.rknn | ~46MB | 人物属性認識 |

## 使用方法

### サービス管理

```bash
# 状態確認
sudo systemctl status is21-infer.service

# ログ確認
sudo journalctl -u is21-infer.service -f

# 再起動
sudo systemctl restart is21-infer.service
```

### API確認

```bash
# ヘルスチェック
curl http://localhost:9000/healthz

# デバイス状態
curl http://localhost:9000/api/status

# 機能一覧
curl http://localhost:9000/v1/capabilities
```

### 画像解析

```bash
curl -X POST http://localhost:9000/v1/analyze \
  -F "infer_image=@camera.jpg" \
  -F "camera_id=cam_2f_201" \
  -F "schema_version=2025-12-29.1" \
  -F "captured_at=2025-12-30T15:30:00Z" \
  -F 'hints_json={
    "location_type": "corridor",
    "distance": "near",
    "expected_objects": ["person"]
  }'
```

## API仕様

### エンドポイント

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| `/healthz` | GET | ヘルスチェック |
| `/api/status` | GET | デバイス状態 |
| `/api/hardware` | GET | ハードウェア情報 |
| `/v1/capabilities` | GET | 機能一覧 |
| `/v1/analyze` | POST | 画像解析 |
| `/v1/schema` | GET/PUT | スキーマ管理 |

### camera_context パラメータ

| パラメータ | 説明 | 例 |
|-----------|------|-----|
| `location_type` | 設置場所 | corridor, entrance, parking, restricted |
| `distance` | カメラ距離 | near, medium, far |
| `excluded_objects` | 除外オブジェクト | ["car", "truck"] |
| `expected_objects` | 期待オブジェクト | ["person"] |
| `busy_hours` | 繁忙時間帯 | ["15:00-18:00"] |
| `quiet_hours` | 静寂時間帯 | ["0:00-6:00"] |

## 更新

```bash
# 最新コードを取得
git pull

# 更新スクリプト実行
sudo ./scripts/update.sh
```

## トラブルシューティング

### サービスが起動しない

```bash
# ログ確認
sudo journalctl -u is21-infer.service -n 50 --no-pager

# モデルファイル確認
ls -la /opt/is21/models/

# 依存パッケージ確認
pip3 list | grep -E "fastapi|uvicorn|numpy|opencv"
```

### RKNN関連エラー

```bash
# rknn-toolkit2-lite確認
python3 -c "from rknnlite.api import RKNNLite; print('OK')"

# NPU確認
cat /sys/class/devfreq/fdab0000.npu/cur_freq
```

## 関連ドキュメント

- `designs/IS21_AI_IMPLEMENTATION_REPORT.md` - AI機能実装報告
- `designs/IS21_PLATFORM_STATUS_REPORT.md` - プラットフォーム状態報告
- `designs/IS22_CAMERA_CONTEXT_GUIDE.md` - camera_contextガイド

## バージョン履歴

| バージョン | 日付 | 主要変更 |
|-----------|------|----------|
| 1.6.0 | 2025-12-29 | 基本PAR実装 |
| 1.7.0 | 2025-12-30 | 人物特徴強化 |
| 1.8.0 | 2025-12-30 | camera_context対応 |

---

作成: Claude Code
