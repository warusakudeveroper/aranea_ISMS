# is20s デプロイパッケージ

Orange Pi Zero3 (Armbian) 向け is20s インストールパッケージ

## パッケージ内容

```
deploy/
├── install.sh      # インストールスクリプト
├── is20s.service   # systemd サービス定義
├── is20s.tar.gz    # アプリケーション本体
└── README.md       # この文書
```

## 前提条件

- Orange Pi Zero3 + Armbian
- ユーザー `mijeosadmin` が存在すること
- インターネット接続（依存パッケージのインストール用）

## インストール手順

### 1. パッケージを転送

```bash
# ローカルPC から Orange Pi へ転送
scp -r deploy/ mijeosadmin@192.168.x.x:/tmp/is20s-install/
```

### 2. インストール実行

```bash
# Orange Pi に SSH 接続
ssh mijeosadmin@192.168.x.x

# インストール実行（sudo 必須）
sudo bash /tmp/is20s-install/install.sh
```

### 3. 動作確認

```bash
# サービス状態確認
sudo systemctl status is20s

# Web UI にアクセス
# ブラウザで http://192.168.x.x:8080 を開く
```

## 一括デプロイ（複数台）

```bash
# デバイスリスト
DEVICES="192.168.3.101 192.168.3.102 192.168.3.103"

# 一括デプロイ
for IP in $DEVICES; do
  echo "=== Deploying to $IP ==="
  scp -r deploy/ mijeosadmin@$IP:/tmp/is20s-install/
  ssh mijeosadmin@$IP "echo 'mijeos12345@' | sudo -S bash /tmp/is20s-install/install.sh"
done
```

## インストール後の設定

### Room マッピング設定

Web UI の Capture タブで設定、または直接編集:

```bash
sudo nano /var/lib/is20s/watch_config.json
```

```json
{
  "rooms": {
    "192.168.3.100": "101",
    "192.168.3.101": "102"
  }
}
```

設定変更後、サービス再起動:

```bash
sudo systemctl restart is20s
```

## トラブルシューティング

### サービスが起動しない

```bash
# ログ確認
sudo journalctl -u is20s -n 50

# 手動起動テスト
cd /opt/is20s
sudo -u mijeosadmin .venv/bin/uvicorn app.main:app --host 0.0.0.0 --port 8080
```

### tshark が動作しない

```bash
# 権限確認
groups mijeosadmin  # wireshark グループに入っているか

# 再設定
sudo usermod -aG wireshark mijeosadmin
sudo setcap cap_net_raw,cap_net_admin=eip /usr/bin/dumpcap

# 再ログイン必要
```

### ポート 8080 が使用中

```bash
# 使用中のプロセス確認
sudo lsof -i :8080

# 別ポートで起動する場合は is20s.service を編集
sudo nano /etc/systemd/system/is20s.service
# --port 8080 を変更
sudo systemctl daemon-reload
sudo systemctl restart is20s
```

## アンインストール

```bash
sudo systemctl stop is20s
sudo systemctl disable is20s
sudo rm /etc/systemd/system/is20s.service
sudo rm -rf /opt/is20s
sudo rm -rf /var/lib/is20s
sudo systemctl daemon-reload
```

## ファイル構成（インストール後）

```
/opt/is20s/                    # アプリケーション
├── app/                       # Python ソースコード
├── requirements.txt           # 依存関係
└── .venv/                     # Python 仮想環境

/var/lib/is20s/                # データディレクトリ
├── watch_config.json          # キャプチャ設定
├── capture_logs/              # キャプチャログ
└── relay.sqlite               # イベントDB

/etc/systemd/system/is20s.service  # systemd 定義
```
