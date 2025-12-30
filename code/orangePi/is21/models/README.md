# IS21 AIモデルファイル

このディレクトリにはRKNN形式のAIモデルファイルを配置します。

## 必要なモデル

| ファイル | サイズ | 用途 |
|---------|--------|------|
| yolov5s-640-640.rknn | ~8MB | 物体検出 (YOLOv5s) |
| par_resnet50_pa100k.rknn | ~46MB | 人物属性認識 (PAR) |

## 注意

- `.rknn` ファイルは `.gitignore` により Git管理対象外です
- モデルファイルは Dropbox 経由で同期されます
- 新規デバイスへは手動でコピーが必要です

## コピー方法

```bash
# ローカルからデバイスへ
scp models/*.rknn user@device:/opt/is21/models/

# デバイス間コピー
scp user@source:/opt/is21/models/*.rknn user@dest:/opt/is21/models/
```

## モデル情報

### yolov5s-640-640.rknn
- 入力: 640x640 RGB
- 出力: COCO 80クラス検出
- 処理時間: 40-70ms (RK3588 NPU)

### par_resnet50_pa100k.rknn
- 入力: 192x256 RGB (人物crop)
- 出力: PA100K 26属性
- 処理時間: 20-30ms (RK3588 NPU)
