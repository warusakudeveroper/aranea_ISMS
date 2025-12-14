以下、**IS03（Orange Pi Zero3）**について「このシステム内での立ち位置・機能」と、**SDへOSを書いてから、セットアップ→コード投入→テスト→ブラウザ確認**までを、**リテラシ低めのスタッフでも迷いにくい**ようにまとめ直した概要手順書です（※is01/is03の送受信関係も正しい形で記載）。

---

# 1) IS03の立ち位置と役割（超重要）

## is01 / is03 / is02 の関係（送受信の向き）

* **is01（電池式 温湿度センサー）**
  温湿度を測って **BLE（Bluetoothの電波）で発信（アドバタイズ）する側**。
  ふだんは **受信しません**（送るだけ）。

* **is03（Orange Pi Zero3）**
  is01が出すBLE電波を **受信して保存し、ブラウザで見える化する側**。
  現地で「is01が動いているか」を確認するための **ローカル受信サーバ**。
  2台置き（冗長化）で、片方が止まっても受信が継続しやすい。

* **is02（常時電源ゲートウェイ）**
  is01のBLEを **受信**し、Wi-Fiでクラウド等へ **送信する側**。
  （サーバ側が遅れている現状では、まずis03で受信確認できればOK）

## IS03がやること（現地運用で必要な機能）

* is01のBLEアドバタイズを受信して、

  * **最新2000件をメモリに保持**（画面に出すのはここ）
  * SDカードには **まとめて保存（バッチ書き込み）**してSDの寿命を守る
* LAN内のブラウザで見えるページを提供

  * 「受信一覧」「動作ログ」「ライブ表示（WS）」程度
* 電源再投入しかできない前提なので、**自動起動・自動復旧**が必須

---

# 2) 今日やる作業の全体像（流れ）

1. **SDカードにOSイメージを書く**
2. **is03にSDを挿して起動**（LANに繋ぐ）
3. **初期セットアップ**（ユーザー作成 / 更新 / Bluetooth）
4. **固定IPを設定**

   * is03-1：`192.168.96.201`
   * is03-2：`192.168.96.202`
5. **is03-relay（受信＆表示アプリ）を入れる**
6. **自動起動設定（systemd）**
7. **テスト（疑似イベント→2000件保持→DB保存→再起動）**
8. **ブラウザで確認**（PCからアクセス）

---

# 3) SDカードにOSイメージを書く（PC作業）

## 使用するイメージ（固定）

* **Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img**

## 書き込み方法（簡単）

1. microSDカードをPCに挿す
2. **balenaEtcher**（推奨）を起動
3. イメージに上記 `.img` を選ぶ
4. 書き込み先に microSD を選ぶ（※間違えると別ディスクが消えます）
5. Flash を押す
6. 完了したら microSDを取り外す

✅チェック

* エラー無しで完了

---

# 4) 起動と配線（現地作業）

1. Orange Pi Zero3 に microSD を挿す
2. **有線LAN（eth0）**を挿す
3. 電源を入れる
4. **2〜3分待つ**

✅チェック

* LANリンクランプが点灯/点滅

---

# 5) 初期セットアップ（最低限）

## 5-1. ユーザー

* ユーザー：`isms`
* パス：`isms12345@`
  （後で変更予定でもOK）

## 5-2. 更新と必要ソフト

端末で以下を実行（コピペOK）：

```bash
sudo apt update && sudo apt -y upgrade
sudo apt -y install bluetooth bluez python3-venv python3-pip sqlite3 jq curl
sudo systemctl enable --now bluetooth
sudo usermod -aG sudo,bluetooth isms
```

✅チェック

```bash
systemctl is-active bluetooth
```

* `active` になっている

---

# 6) 固定IP設定（is03-1 / is03-2）

固定IPは必須です。

* is03-1：`192.168.96.201/23`
* is03-2：`192.168.96.202/23`

> 最初だけDHCPで起動し、**ゲートウェイとDNSを自動取得**してから固定化すると事故が減ります。

## 6-1. ゲートウェイとDNSを確認

```bash
GW="$(ip route | awk '/default/ {print $3; exit}')"
DNS1="$(awk '/^nameserver /{print $2; exit}' /etc/resolv.conf)"
echo "GW=$GW"
echo "DNS1=$DNS1"
```

## 6-2. 固定IPを適用（NetworkManagerが普通なので nmcli 版）

is03-1 なら `IP_ADDR=192.168.96.201`、is03-2 なら `192.168.96.202` にして実行：

```bash
IP_ADDR="192.168.96.201"   # is03-2 は 192.168.96.202
CONN="$(nmcli -t -f NAME,DEVICE con show --active | awk -F: '$2=="eth0"{print $1; exit}')"
sudo nmcli con mod "$CONN" ipv4.method manual
sudo nmcli con mod "$CONN" ipv4.addresses "${IP_ADDR}/23"
sudo nmcli con mod "$CONN" ipv4.gateway "$GW"
sudo nmcli con mod "$CONN" ipv4.dns "$DNS1"
sudo nmcli con mod "$CONN" ipv4.ignore-auto-dns yes
sudo nmcli con down "$CONN" || true
sudo nmcli con up "$CONN"
```

✅チェック

```bash
ip -4 addr show dev eth0
ping -c 3 "$GW"
```

* IPが .201 または .202 になっている
* GWにpingが返る

---

# 7) is03-relay（受信＆表示アプリ）の投入（概要）

is03には「受信→保存→ブラウザ表示」するアプリ **is03-relay**を入れます。

## 7-1. アプリの役割

* BLE受信を続ける
* **最新2000件をメモリに保持**（画面表示用）
* SDカードへは **まとめてSQLite保存**（例：10秒ごと/50件ごとに一括）
* DBは肥大化防止（例：最大20000件で古いのを削除）
* ブラウザ表示：

  * `http://<is03>:8080/`
  * `受信一覧 / ログ / ライブ表示`

## 7-2. 自動起動（超重要）

電源再投入しかできない前提なので、**systemdで自動起動**になっていることが絶対条件です。

* 起動に失敗しても `Restart=always` で自動復旧

---

# 8) テスト（ここまでできれば現地投入OK）

## 8-1. 起動確認

```bash
sudo systemctl status is03-relay --no-pager
```

* `active (running)` ならOK

## 8-2. アプリ動作確認（ブラウザ無しでも確認できる）

```bash
curl -sS http://localhost:8080/api/status
```

* `ok:true` が含まれるならOK

## 8-3. 疑似イベントで動作確認（BLEが無くてもOK）

```bash
curl -sS -X POST http://localhost:8080/api/debug/publishSample
curl -sS "http://localhost:8080/api/events?limit=5"
```

* eventsが増える

## 8-4. 「最新2000件保持」の確認（重要）

```bash
for i in $(seq 1 2500); do curl -sS -X POST http://localhost:8080/api/debug/publishSample >/dev/null; done
curl -sS "http://localhost:8080/api/events?limit=5000" | jq 'length'
```

* **2000** になっていればOK（超えない）

## 8-5. DB保存されているか確認（SDへまとめ書き）

```bash
ls -lh /var/lib/is03-relay/relay.sqlite
sqlite3 /var/lib/is03-relay/relay.sqlite "select count(*) from events;"
```

* countが0より大きい

---

# 9) ブラウザ確認（最終）

同じLANのPCからアクセス：

* is03-1：**[http://192.168.96.201:8080/](http://192.168.96.201:8080/)**
* is03-2：**[http://192.168.96.202:8080/](http://192.168.96.202:8080/)**

✅チェック

* 画面が開く
* 受信一覧が出る
* is01が近くで動いていれば、数分以内に受信が増える

---

# 10) 電源再投入前提の確認（必須）

```bash
sudo reboot
```

再起動後：

* ブラウザがまた開く
* `is03-relay` が勝手に起動している

確認コマンド：

```bash
systemctl is-active is03-relay
curl -sS http://localhost:8080/api/status
```

---

