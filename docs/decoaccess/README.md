# TP-Link DECO Router API Access

TP-Link DECOルーターからAPI経由で情報を取得するツール。

## 概要

DECOルーターはRSA + AES暗号化された独自APIを使用しており、curlでの直接アクセスは困難。
本ツールは `tplinkrouterc6u` ライブラリを使用して認証・暗号化を処理する。

## 取得可能な情報

- ネットワーク情報（WAN/LAN IP, MAC, Gateway）
- システム状態（CPU使用率, メモリ使用率）
- WiFi状態（2.4GHz/5GHz/6GHz有効・無効）
- 接続クライアント一覧（ホスト名, IP, MAC, 接続バンド）
- ファームウェア情報

## セットアップ

### 1. 依存ライブラリのインストール

```bash
pip3 install tplinkrouterc6u
```

### 2. 設定ファイルの編集

`config.json` を編集してルーターの接続情報を設定:

```json
{
  "host": "192.168.68.1",
  "password": "your_password_here",
  "username": "admin"
}
```

## 使い方

### 基本的な使用方法

```bash
# 人間が読みやすい形式で出力
python3 deco_monitor.py

# JSON形式で出力
python3 deco_monitor.py --json

# 別の設定ファイルを使用
python3 deco_monitor.py --config /path/to/config.json

# コマンドラインでホスト/パスワードを指定
python3 deco_monitor.py --host 192.168.1.1 --password mypassword
```

### 出力例（テキスト形式）

```
======================================================================
TP-Link DECO Router Status - 2026-01-07T14:24:26.432965
======================================================================

[Network]
  WAN MAC:     48-22-54-AA-3A-49
  WAN IP:      114.187.7.63
  WAN Gateway: 125.201.245.142
  LAN MAC:     48-22-54-AA-3A-48
  LAN IP:      192.168.68.1

[System]
  CPU Usage:    3.0%
  Memory Usage: 38.0%

[Clients]
  WiFi:   4
  Wired:  0
  Guest:  0
  Total:  4

[Connected Devices]
----------------------------------------------------------------------
Hostname                     IP Address       MAC Address        Band
----------------------------------------------------------------------
SwitchBot HubMini            192.168.68.51    30-83-98-BC-9F-AA  2.4GHz
espressif                    192.168.68.52    30-83-98-42-C1-64  2.4GHz
----------------------------------------------------------------------
Total: 2 devices
```

### 出力例（JSON形式）

```json
{
  "timestamp": "2026-01-07T14:24:46.977068",
  "network": {
    "wan_mac": "48-22-54-AA-3A-49",
    "wan_ip": "114.187.7.63",
    "lan_ip": "192.168.68.1"
  },
  "system": {
    "cpu_usage": 3.0,
    "mem_usage": 37.0
  },
  "clients": {
    "wifi_total": 4,
    "total": 4
  },
  "devices": [
    {
      "hostname": "SwitchBot HubMini",
      "ip": "192.168.68.51",
      "mac": "30-83-98-BC-9F-AA",
      "connection": "2.4GHz"
    }
  ]
}
```

## 定期監視（cron設定例）

5分ごとにJSONを保存:

```bash
*/5 * * * * cd /path/to/decoaccess && python3 deco_monitor.py --json > /var/log/deco_status.json 2>&1
```

## API技術詳細

### 認証フロー

1. `/cgi-bin/luci/;stok=/login?form=keys` - 1024bit RSA公開鍵取得（パスワード暗号化用）
2. `/cgi-bin/luci/;stok=/login?form=auth` - 512bit RSA公開鍵+seq取得（署名用）
3. `/cgi-bin/luci/;stok=/login?form=login` - ログイン（暗号化パスワード送信）
4. セッショントークン(stok)とsysauth Cookieを取得

### データ取得エンドポイント

| エンドポイント | 説明 |
|---------------|------|
| `admin/network?form=wan_ipv4` | WAN/LAN IP情報 |
| `admin/network?form=performance` | CPU/メモリ使用率 |
| `admin/wireless?form=wlan` | WiFi状態 |
| `admin/client?form=client_list` | 接続クライアント |
| `admin/device?form=device_list` | Decoメッシュノード |

### 暗号化方式

- パスワード: RSA PKCS#1 v1.5 (1024bit)
- リクエストボディ: AES-128-CBC (Base64エンコード)
- 署名: RSA (512bit) で `k=KEY&i=IV&h=HASH&s=SEQ+LEN` を暗号化

## トラブルシューティング

### 接続エラー

- ルーターのIPアドレスが正しいか確認
- 同一ネットワーク上にいるか確認
- ファイアウォールでHTTP(80)がブロックされていないか確認

### 認証エラー

- パスワードが正しいか確認
- ルーター管理画面にログインできるか確認

### ライブラリエラー

```bash
pip3 install --upgrade tplinkrouterc6u pycryptodome
```

## 参考資料

- [tplinkrouterc6u - PyPI](https://pypi.org/project/tplinkrouterc6u/)
- [TP-Link Deco API Example (GitHub Gist)](https://gist.github.com/rosmo/29200c1aedb991ce55942c4ae8b54edd)
