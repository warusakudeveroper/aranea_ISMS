【PROMPT / Armbian is03（Orange Pi Zero3）構築・登録・受信表示（Claude Code / Cursor / AI作業者向け）】
あなたはAIコーディングエージェントです。以下を「上から順に」「省略せず」「推測せず」実行してください。不足情報があれば作業を停止し、必要な値を私に要求してください。完了条件を満たすまで終了しないこと。

============================================================
0) 目的（このプロンプトのゴール）
============================================================
- is03（Orange Pi Zero3）を2台（is03-1 / is03-2）構築する。
- is03は「is01のBLEアドバタイズ（温湿度など）を受信してローカル保存し、LANブラウザで見える化」する。
- 最新イベントを「オンメモリで最大2000件」保持する（/api/events はここから返す）。
- SDカード保護のため、DB（SQLite）への書き込みは「バッチ（まとめ書き）」で行い、頻繁に書かない。
- is03は固定IPを設定する：
  - is03-1: 192.168.96.201/23
  - is03-2: 192.168.96.202/23
- 電源再投入しかできない前提：OS起動→ネット→is03-relay起動まで自動（systemdで自動起動・自動復旧）。
- is03自身を araneaDevice として araneaDeviceGate に登録し、cic_codeを取得してローカル設定に保存する。
- ブラウザで “受信画面” が表示できるところまでテストする。

============================================================
1) 固定値（変更禁止）
============================================================
# 使用するOSイメージ（必ずこれ）
ARMBIAN_IMAGE="Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img"

# テナント（市山水産株式会社）
TENANT_NAME="市山水産株式会社"
TID="T2025120608261484221"
FID="9000"

# テナントプライマリ（perm61）
TENANT_PRIMARY_EMAIL="info+ichiyama@neki.tech"
TENANT_PRIMARY_CIC="263238"
TENANT_PRIMARY_PASS="dJBU^TpG%j$5"
# IMPORTANT: TENANT_PRIMARY_LACISID(20文字) はこの入力に含まれない。
# 登録に必須なので、必ず取得して環境変数にセットする。
# 取得できない場合は停止して私に要求すること。

# is03 デバイス固定値
DEVICE_TYPE="ISMS_ar-is03"
PRODUCT_TYPE="003"
PRODUCT_CODE="0096"

# is03 固定IP
IS03_1_IP="192.168.96.201"
IS03_2_IP="192.168.96.202"
CIDR="/23"
SUBNET="192.168.96.0/23"
NETMASK="255.255.254.0"

# mobes2.0 実エンドポイント（例文禁止。これを使用し、到達確認する）
ARANEA_DEVICE_GATE_URL="https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
DEVICE_STATE_REPORT_URL="https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"

# is03の lacisId はこの形式でOK（20文字）
# lacisId = "3" + "003" + MAC12(大文字HEX,コロンなし12桁) + "0096"
# 例: 3003AABBCCDDEEFF0096

============================================================
2) 事前に必要な追加入力（無い場合は必ず停止）
============================================================
(必須) TENANT_PRIMARY_LACISID（20文字）
- 取得方法:
  - mobes2.0 側で Firestore から Tenant Primary（info+ichiyama@neki.tech, tid一致, perm>=61）を検索して doc.id を特定する。
  - それを TENANT_PRIMARY_LACISID として渡す。
- ここで分からない場合は作業を止めて「TENANT_PRIMARY_LACISID をください」と要求すること。

============================================================
3) SDカードにOSイメージを書き込む（作業指示・省略禁止）
============================================================
- PCで balenaEtcher 等を使用し、ARMBIAN_IMAGE を microSD に書き込む。
- 書き込み完了後、microSDを is03 に挿入し、有線LAN（eth0）を接続し電源投入する。

============================================================
4) 初回起動セットアップ（is03-1 / is03-2 両方で実施）
============================================================
4-1) 初回ユーザー作成（Armbian初回ウィザードが出る場合）
- ユーザー: isms
- パスワード: isms12345@
（ウィザードが出ず root しか入れない等の場合は状況をログに残し、同等の状態（isms作成）を必ず実現する）

4-2) hostname 設定
- 1台目: is03-1
- 2台目: is03-2
実行:
sudo hostnamectl set-hostname is03-1   # is03-2側は is03-2
hostname

4-3) パッケージ導入（軽量・必要最小）
sudo apt update && sudo apt -y upgrade
sudo apt -y install bluetooth bluez python3-venv python3-pip sqlite3 jq curl
sudo systemctl enable --now bluetooth

4-4) isms を sudo / bluetooth グループへ
sudo usermod -aG sudo,bluetooth isms

============================================================
5) 固定IP設定（必ず「DHCPでGW/DNS取得→固定化」の順で実施）
============================================================
5-1) DHCPで起動中にGW/DNSを取得（必須）
GW="$(ip route | awk '/default/ {print $3; exit}')"
DNS1="$(awk '/^nameserver /{print $2; exit}' /etc/resolv.conf)"
echo "GW=$GW"
echo "DNS1=$DNS1"
停止条件：GW または DNS1 が空なら停止して私に GW/DNS を要求すること。

5-2) NetworkManager / netplan どちらかを判定
systemctl is-active NetworkManager && echo "NM=active" || echo "NM=inactive"
ls -1 /etc/netplan/*.yaml 2>/dev/null || true

5-3) is03-1 / is03-2 のどちらかに合わせてIPを決める（必須）
- is03-1 → IP="$IS03_1_IP"
- is03-2 → IP="$IS03_2_IP"
IP_ADDR="(この機体のIP)"

5-4) 固定IPを適用
(A) NetworkManager が active の場合（nmcliで設定）
CONN="$(nmcli -t -f NAME,DEVICE con show --active | awk -F: '$2=="eth0"{print $1; exit}')"
if [ -z "$CONN" ]; then CONN="$(nmcli -t -f NAME,DEVICE con show | awk -F: '$2=="eth0"{print $1; exit}')"; fi
停止条件：CONN が空なら停止して原因をログ化すること。

sudo nmcli con mod "$CONN" ipv4.method manual
sudo nmcli con mod "$CONN" ipv4.addresses "${IP_ADDR}${CIDR}"
sudo nmcli con mod "$CONN" ipv4.gateway "$GW"
sudo nmcli con mod "$CONN" ipv4.dns "$DNS1"
sudo nmcli con mod "$CONN" ipv4.ignore-auto-dns yes
sudo nmcli con down "$CONN" || true
sudo nmcli con up "$CONN"

(B) NetworkManager が inactive かつ netplan がある場合
sudo tee /etc/netplan/99-is03-static.yaml >/dev/null <<YAML
network:
  version: 2
  renderer: networkd
  ethernets:
    eth0:
      dhcp4: no
      addresses: [${IP_ADDR}/23]
      routes:
        - to: default
          via: ${GW}
      nameservers:
        addresses: [${DNS1}]
YAML
sudo netplan apply

5-5) 確認（必須）
ip -4 addr show dev eth0
ip route | head -n 30
ping -c 3 "$GW"
停止条件：失敗したら停止し原因をログ化する。

============================================================
6) is03-relay（BLE受信→メモリリング2000→SQLiteへバッチ書き込み→HTTP/WS表示）を実装
============================================================
6-1) 配置場所（固定）
- アプリ: /opt/is03-relay
- DB: /var/lib/is03-relay/relay.sqlite
- 設定: /etc/is03-relay/config.json

作成:
sudo mkdir -p /opt/is03-relay /opt/is03-relay/app /opt/is03-relay/app/static /var/lib/is03-relay /etc/is03-relay
sudo chown -R isms:isms /opt/is03-relay /var/lib/is03-relay

6-2) venvと依存導入
sudo -u isms python3 -m venv /opt/is03-relay/venv
sudo -u isms /opt/is03-relay/venv/bin/pip install --upgrade pip
sudo -u isms /opt/is03-relay/venv/bin/pip install fastapi uvicorn[standard] websockets bleak aiosqlite

6-3) SQLiteスキーマ（固定）
テーブル events（永続保存用）:
- id INTEGER PRIMARY KEY AUTOINCREMENT
- seenAt TEXT NOT NULL             # ISO8601 UTC
- src TEXT NOT NULL                # "ble" or "debug"
- mac TEXT NOT NULL                # "AABBCCDDEEFF"
- rssi INTEGER
- adv_hex TEXT
- manufacturer_hex TEXT
- lacisId TEXT
- productType TEXT
- productCode TEXT

起動時PRAGMA（必須）:
- journal_mode=WAL
- synchronous=NORMAL
- temp_store=MEMORY

DB保存の上限（必須）:
- 永続DBは最大 20000件
- 超えたら古い順に削除する（flush後に実施、毎回ではなく定期で良い）

6-4) メモリ保持（必須仕様）
- collections.deque(maxlen=2000) を inMemoryRing として保持する（最新2000件）
- /api/events はDBではなく inMemoryRing から返す（最新順・limit対応）
- SD保護のため、受信イベントは即DBへ書かず、writeQueueに積む
- flush条件（必須・固定）:
  - 10秒ごと OR 50件たまったら一括INSERT（transaction）
- flush後に lastFlushAt を更新

6-5) HTTP/WS（固定）
- GET  /api/status
  -> JSON: { ok:true, hostname, uptimeSec, bluetooth:"ok|ng", db:"ok|ng", ringSize, queuedWrites, lastFlushAt }
- GET  /api/events?limit=200
  -> inMemoryRing の最新をJSON配列で返す（最大2000、limit上限は2000）
- GET  /
  -> index.html（簡易UI。イベント一覧とWS接続状態が分かる）
- WS   /ws
  -> 新規イベントが入ったらJSONをpush
- POST /api/debug/publishSample
  -> debugイベントを1件投入（ring/queue/flush/WSすべて通す。BLE無しでもテスト可能にする）

6-6) BLEスキャン（必須）
- bleak で常時スキャン
- 受信ごとに event を生成し enqueue_event(event) を呼ぶ
- mac は “AABBCCDDEEFF（大文字/コロン無し）” で保存
- lacisId は is03側では暫定で次を生成して保存して良い：
  lacisId="3"+"003"+mac+"0096"
  ※is01側の正式payloadが確定したら将来改善するが、現時点の疎通確認目的ではこれで良い。

6-7) 実装ファイル（必須・このパスで作成）
- /opt/is03-relay/app/main.py      # FastAPI起動・lifespanでBLE+flush task開始
- /opt/is03-relay/app/db.py        # ring/queue/flush/SQLite
- /opt/is03-relay/app/ble.py       # BLE scanner
- /opt/is03-relay/app/web.py       # router + websocket manager
- /opt/is03-relay/app/static/index.html

============================================================
7) systemd 自動起動（電源再投入前提：必須）
============================================================
7-1) service作成（必須）
sudo tee /etc/systemd/system/is03-relay.service >/dev/null <<'SERVICE'
[Unit]
Description=is03-relay (BLE -> RAM ring(2000) -> SQLite batch -> HTTP/WS)
After=network-online.target bluetooth.service
Wants=network-online.target

[Service]
WorkingDirectory=/opt/is03-relay
ExecStartPre=/bin/sh -c 'sleep 3'
ExecStart=/opt/is03-relay/venv/bin/uvicorn app.main:app --host 0.0.0.0 --port 8080
Restart=always
RestartSec=2
User=isms

[Install]
WantedBy=multi-user.target
SERVICE

sudo systemctl daemon-reload
sudo systemctl enable --now is03-relay
sudo systemctl status is03-relay --no-pager

7-2) journaldサイズ制限（SD保護：推奨だが実施する）
sudo sed -i 's/^#SystemMaxUse=.*/SystemMaxUse=100M/' /etc/systemd/journald.conf || true
sudo sed -i 's/^#RuntimeMaxUse=.*/RuntimeMaxUse=50M/' /etc/systemd/journald.conf || true
sudo systemctl restart systemd-journald

============================================================
8) is03 を araneaDeviceGate に登録（必須：成功するまで）
============================================================
8-1) TENANT_PRIMARY_LACISID をセット（必須）
export TENANT_PRIMARY_LACISID="(20文字)"
停止条件：空なら停止して私に要求。

8-2) eth0 の MAC12 を取得して is03 lacisId を生成
MAC_RAW="$(cat /sys/class/net/eth0/address 2>/dev/null || cat /sys/class/net/wlan0/address)"
MAC12="$(echo "$MAC_RAW" | tr -d ':' | tr '[:lower:]' '[:upper:]')"
IS03_LACISID="3${PRODUCT_TYPE}${MAC12}${PRODUCT_CODE}"
echo "$IS03_LACISID"
# 20文字チェック
python3 - <<PY
s="$IS03_LACISID"
assert len(s)==20
PY

8-3) araneaDeviceGate に登録（必須）
curl -sS -D /tmp/hdr_is03.txt -o /tmp/body_is03.json \
  -X POST "$ARANEA_DEVICE_GATE_URL" \
  -H "Content-Type: application/json" \
  -H "X-LacisOath-Version: 1" \
  -d "$(cat <<JSON
{
  "lacisOath": {
    "lacisId": "$TENANT_PRIMARY_LACISID",
    "userId": "info+ichiyama@neki.tech",
    "pass": "dJBU^TpG%j$5",
    "cic": "263238",
    "method": "register"
  },
  "userObject": {
    "tid": "T2025120608261484221",
    "typeDomain": "araneaDevice",
    "type": "ISMS_ar-is03",
    "permission": 11
  },
  "deviceMeta": {
    "macAddress": "$MAC12",
    "productType": "003",
    "productCode": "0096"
  }
}
JSON
)"
cat /tmp/hdr_is03.txt
cat /tmp/body_is03.json

8-4) 返却値チェック（必須）
NEW_LACISID="$(jq -r '.lacisId' /tmp/body_is03.json)"
NEW_CIC_CODE="$(jq -r '.userObject.cic_code' /tmp/body_is03.json)"
python3 - <<PY
expected="$IS03_LACISID"
actual="$NEW_LACISID"
assert expected == actual
assert "$NEW_CIC_CODE".isdigit() and len("$NEW_CIC_CODE")==6
PY

8-5) 設定に保存（必須）
sudo tee /etc/is03-relay/config.json >/dev/null <<JSON
{
  "tenantName":