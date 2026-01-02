# is21-22 テスト環境接続情報

作成日: 2025-12-30

## is21 (camimageEdge AI)

| 項目 | 設定値 |
|------|--------|
| IP | 192.168.3.116 |
| Hostname | araneais21camimageEdgeAI |
| Pretty Hostname | aranea_is21camimageEdgeAI |
| Username | mijeosadmin |
| Password | mijeos12345@ |
| root Password | mijeos12345@ |
| SSH | 自動起動有効（enabled） |
| OS | Armbian 25.11.2 noble |
| Kernel | Linux 6.1.115-vendor-rk35xx |
| API Port | 9000 |

### 接続コマンド

```bash
# SSH接続
ssh mijeosadmin@192.168.3.116
# password: mijeos12345@

# sudo実行
echo 'mijeos12345@' | sudo -S <command>

# API確認
curl http://192.168.3.116:9000/api/status
```

### 推論API

```bash
# 画像推論
curl -X POST http://192.168.3.116:9000/v1/analyze \
  -F "camera_id=test01" \
  -F "captured_at=2025-12-30T00:00:00" \
  -F "schema_version=2025-12-29.1" \
  -F "infer_image=@/path/to/image.jpg"
```

### サービス管理

```bash
# サービス再起動
sudo systemctl restart is21-infer.service

# ログ確認
sudo journalctl -u is21-infer -f
```

### ファイルパス

| パス | 内容 |
|------|------|
| /opt/is21/src/main.py | 推論サーバ本体 |
| /opt/is21/models/ | RKNNモデル |
| /opt/is21/config/ | 設定ファイル |

---

## is22 (予定)

*未セットアップ*
