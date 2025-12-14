# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ISMS（情報セキュリティマネジメントシステム）向けIoTデバイス開発プロジェクト。離島・VPN運用を前提とした温湿度監視・接点制御システム。

## System Architecture

### Device Roles (is01〜is05)

- **is01**: 電池式温湿度センサー。BLEアドバタイズで送信のみ（受信しない）。DeepSleep運用。OTA不可で固定。HT-30センサー搭載。
- **is02**: 常時電源ゲートウェイ。is01のBLEを受信し、Wi-Fi経由でis03へHTTP中継。自身もHT-30センサーを持ち、30秒ごとにバッチ送信。OTA/Web UI対応。
- **is03**: Orange Pi Zero3。is02からHTTP POST経由でデータ受信、ローカル保存・ブラウザ表示。冗長化のため2台構成。
  - is03-1: `192.168.96.201:8080` (primary)
  - is03-2: `192.168.96.202:8080` (secondary)
- **is04**: 接点出力デバイス（GPIO12/14制御）。ローカルMQTT/HTTP経由で制御。
- **is05**: 8chリード入力。窓/ドアの開閉状態を検知してis03へ送信。

### Data Flow

```
is01 (BLE Advertise) → is02 (BLE Scan) → is02 (HTTP POST) → is03 (保存・表示)
                                               ↓
is02自身のHT-30センサー ────────────────────→ is03

is04/is05 ←→ is03 (ローカルMQTT/HTTP)
```

**重要**: is01のBLEデータは is02 が中継する。is03は is02 からのHTTP POSTのみを受信する。is03のBLEスキャン機能は参考実装として残っているが、本番では使用しない。

### LacisID Format

`[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁]` = 20文字
例: `3` + `001` + `AABBCCDDEEFF` + `0096`

## ESP32 Development

### MCP Server

`mcp-arduino-esp32` が設定済み。Arduino CLIでのビルド・書き込みに使用。

外部システム連携の詳細: `docs/common/external_systems.md`

### Port Detection

ESP32接続ポートの確認:
```bash
# Windows
Bash: mode

# Linux/macOS
Bash: ls /dev/tty*
```

通常は `COM3` (Windows) または `/dev/ttyUSB0` (Linux) が使用される。

### Build Commands

Arduino CLIを使用（MCP経由）:
```bash
# is01のビルド・書き込み
mcp__ide__executeCode(language: "arduino", code: "<is01.ino内容>", sketch_path: "code/ESP32/is01")
mcp__arduino_compile(sketch_path: "code/ESP32/is01/is01.ino", fqbn: "esp32:esp32:esp32")
mcp__arduino_upload(sketch_path: "code/ESP32/is01/is01.ino", port: "COM3")

# is02のビルド・書き込み
mcp__arduino_compile(sketch_path: "code/ESP32/is02/is02.ino", fqbn: "esp32:esp32:esp32")
mcp__arduino_upload(sketch_path: "code/ESP32/is02/is02.ino", port: "COM3")

# シリアルモニタ起動
mcp__arduino_monitor_start(port: "COM3")

# ピン使用状況のチェック（実装前推奨）
mcp__arduino_pin_check(sketch_path: "code/ESP32/is01/is01.ino")
```

**注意**: MCP経由でコンパイル時、共通ライブラリ（code/ESP32/global/）は自動的にインクルードパスに追加される。

### AraneaDeviceGate Registration

is02/is04/is05の初回起動時、araneaDeviceGateへ登録してCICコードを取得:
- Endpoint: `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`
- Method: POST with lacisOath authentication
- Returns: `cic_code` (6桁)、デバイスに保存される

登録に必要な情報:
- Tenant TID: `T2025120608261484221`
- Tenant Primary LacisID: `12767487939173857894`
- Tenant Email: `info+ichiyama@neki.tech`
- Tenant CIC: `263238`
- Tenant Password: `dJBU^TpG%j$5`

### Common Modules (code/ESP32/global/src/)

| Module | is01 | is02/04/05 | Purpose |
|--------|------|------------|---------|
| LacisIDGenerator | ○ | ○ | STA MACからlacisID生成 |
| AraneaRegister | △(初回のみ) | ○ | cic_code取得・保存 |
| WiFiManager | △(初回のみ) | ○ | cluster1-6への接続試行 |
| DisplayManager | ○ | ○ | I2C OLED 128x64表示 |
| NtpManager | × | ○ | 時刻同期 |
| SettingManager | ○(NVS) | ○(NVS) | 設定永続化 |
| HttpRelayClient | × | ○ | is03へのHTTP POST（primary/secondary フォールバック）|
| BleIsmsParser | × | ○ | is01 BLEペイロード解析 |
| HttpManager | × | ○ | Web UI・API |
| OtaManager | × | ○ | OTA更新 |
| Operator | ○ | ○ | 状態機械・競合制御 |
| RebootScheduler | × | ○ | 定期再起動（設定時刻） |
| SPIFFSManager | × | ○ | ログ保存用 |

### is01 Critical Notes

- I2C初期化でコケやすい。**I2C処理は必ず直列実行**（並列禁止）
- DeepSleep復帰→I2C初期化の順序がシビア
- CIC未取得でもBLE広告は必ず出す（is02で検知可能にする）
- GPIO5をHIGHに設定してI2C用MOSFETを有効化

### is02 Architecture

**バッチ送信モデル（30秒間隔）**:
1. BLEスキャンでis01からデータ受信 → バッチバッファに追加
2. is02自身のHT-30センサー読み取り → バッチバッファに追加（is02も独立した温湿度センサーを持つ）
3. 30秒ごとにバッファ内の全イベントをis03へHTTP POST（BLEイベント + is02自身のセンサーイベントを混在送信）
4. primary失敗時はsecondaryへフォールバック

**重要**: is02は2つの役割を持つ:
- **BLEゲートウェイ**: is01のBLE広告を受信・中継（`meta.direct: false`, `meta.rssi`, `meta.gatewayId`付き）
- **温湿度センサー**: 自身のHT-30センサーで30秒ごとに計測（`meta.direct: true`, rssi/gatewayId無し）

**JSON形式（is02 → is03）**:
```json
{
  "sensor": {
    "mac": "AABBCCDDEEFF",
    "productType": "001",
    "productCode": "0096",
    "lacisId": "3001AABBCCDDEEFF0096"
  },
  "state": {
    "temperature": 23.5,
    "humidity": 65.2,
    "battery": 85
  },
  "meta": {
    "observedAt": "2025-12-14T05:46:14Z",
    "direct": true,              // is02自身の計測
    "rssi": -65,                 // BLEリレー時のみ
    "gatewayId": "30026CC840..."  // BLEリレー時のみ
  }
}
```

### Default WiFi

SSID: `cluster1`〜`cluster6`, Password: `isms12345@`

### Relay Endpoints (is03)

is02のデフォルト設定:
- primary: `http://192.168.96.201:8080/api/events`
- secondary: `http://192.168.96.202:8080/api/events`

NVSキー:
- `relay_pri`: primary URL
- `relay_sec`: secondary URL

## Orange Pi (is03) Development

### Environment

- OS: Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img
- User: `isms`, Password: `isms12345@`
- SSH: 有効（sudo権限あり）

### SSH Access (from Windows with WSL)

```bash
# SSH接続
wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201

# sudo実行（パスワード付き）
wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201 'echo isms12345@ | sudo -S <command>'
```

### Application Structure

```
/opt/is03-relay/
├── venv/                    # Python仮想環境
├── app/
│   ├── main.py             # FastAPI起動・lifespan管理
│   ├── db.py               # メモリリング・SQLite管理
│   ├── ble.py              # BLEスキャナー（参考実装）
│   ├── web.py              # HTTP/WebSocketルーター
│   └── static/
│       └── index.html      # ブラウザUI
/var/lib/is03-relay/
└── relay.sqlite            # SQLiteデータベース
/etc/is03-relay/
└── config.json             # デバイス設定
/etc/systemd/system/
└── is03-relay.service      # systemdサービス
```

### Service Management

```bash
# サービス状態確認
sudo systemctl status is03-relay

# サービス再起動
sudo systemctl restart is03-relay

# ログ確認
sudo journalctl -u is03-relay -n 50

# リアルタイムログ
sudo journalctl -u is03-relay -f
```

### API Endpoints

```bash
# ステータス確認
curl http://192.168.96.201:8080/api/status

# イベント取得（最新200件、最大2000件）
curl http://192.168.96.201:8080/api/events?limit=200

# is02からのデータ受信（POST）
curl -X POST http://192.168.96.201:8080/api/events \
  -H "Content-Type: application/json" \
  -d '{"sensor":{...}, "state":{...}, "meta":{...}}'

# デバッグ用サンプルイベント投入
curl -X POST http://192.168.96.201:8080/api/debug/publishSample
```

### Data Storage

- **メモリリング**: 最新2000件保持（deque、画面表示用）
- **SQLite**: バッチ書き込み（10秒ごと or 50件ごと）
  - パス: `/var/lib/is03-relay/relay.sqlite`
  - 最大保持: 20000件
  - WALモード

### Deployment Workflow

#### is03へのコード更新

```bash
# web.pyの更新例
wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201 'cat > /opt/is03-relay/app/web.py' << 'EOF'
<ファイル内容>
EOF

# サービス再起動
wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201 'echo isms12345@ | sudo -S systemctl restart is03-relay'
```

#### ESP32デバイスの一括書き込み（量産時）

```bash
# Python esptoolを使用した一括フラッシュ
# 192.168.96.201, 192.168.96.202の両方にテストイベント送信（冗長化確認）
for i in {1..10}; do curl -s -X POST http://192.168.96.201:8080/api/debug/publishSample; done
for i in {1..10}; do curl -s -X POST http://192.168.96.202:8080/api/debug/publishSample; done

# 複数ESP32への並列OTA（is02の例）
# esptool経由での初回書き込み後、OTA対応デバイスはIPアドレス指定で更新可能
```

### Browser UI

- URL: `http://192.168.96.201:8080` または `http://192.168.96.202:8080`
- WebSocket自動再接続
- リアルタイム表示（温度・湿度・バッテリー）
- BLEイベントフィルタ機能（"Hide BLE events"チェックボックス）

## Important Implementation Notes

### is02のJSONフォーマット変更

is02は最新バージョン（is02.ino:1-435）で以下の形式でデータ送信:
- `sensor.productType`: "001", "002"（文字列、3桁ゼロパディング）
- `state.temperature/humidity/battery`: キー名が短縮形（temperatureC/humidityPct ではない）
- `meta.direct`: is02自身の計測時に true
- `meta.rssi/gatewayId`: BLEリレー時のみ存在

is03のweb.py:65-93は両方の命名規則に対応:
```python
'temperatureC': state.get('temperatureC') or state.get('temperature')
'humidityPct': state.get('humidityPct') or state.get('humidity')
'batteryPct': state.get('batteryPct') or state.get('battery')
```

### is03 Browser UI Filter

index.html:43-45でBLEイベントフィルタ実装:
- チェックボックスON → BLEイベント非表示（is02の直接計測データのみ表示）
- `row.dataset.source`属性でソース判定（"ble" vs "is02"）
- リアルタイムイベントにも自動適用
- **用途**: BLEリレーデータをフィルタして、is02自身のHT-30センサーデータのみを確認したい場合に有用

### Device Credentials

テナント情報（市山水産株式会社）:
- TID: `T2025120608261484221`
- TENANT_PRIMARY_LACISID: `12767487939173857894`
- TENANT_PRIMARY_EMAIL: `info+ichiyama@neki.tech`
- TENANT_PRIMARY_CIC: `263238`
- TENANT_PRIMARY_PASS: `dJBU^TpG%j$5`

is03デバイス情報:
- is03-1: MAC `0200A846FAC5`, lacisId `30030200A846FAC50096`, cic_code `285135`
- is03-2: MAC `0200A8722742`, lacisId `30030200A87227420096`, cic_code `537533`

### Common Pitfalls

1. **is02のURL設定**: NVSに保存されたURL（`relay_pri`/`relay_sec`）が優先される。コード内のデフォルト値は初回起動時のみ適用。
2. **is03のendpoint**: `/api/events` と `/api/ingest` の両方が実装されている（同じハンドラを使用）。
3. **BLE vs HTTP**: is03のBLEスキャン（ble.py）は参考実装。本番データフローはis02→is03のHTTP POST。
4. **JSON keys**: is02が送信するキー名（temperature/humidity/battery）とis03の内部キー名（temperatureC/humidityPct/batteryPct）が異なるため、両対応が必要。

### Factory Reset

is02/is04/is05でGPIO26長押しによる初期化機能:
- SPIFFS/NVS全消去
- WiFi設定・CIC・リレーURLをデフォルトに戻す
- OLEDに "Factory Reset" 表示後、再起動

## Testing

### is03へのテストデータ送信

```bash
# is02形式のテストデータ
curl -X POST http://192.168.96.201:8080/api/events \
  -H "Content-Type: application/json" \
  -d '{
    "sensor": {
      "mac": "AABBCCDDEEFF",
      "productType": "001",
      "productCode": "0096",
      "lacisId": "3001AABBCCDDEEFF0096"
    },
    "state": {
      "temperature": 25.5,
      "humidity": 60.0,
      "battery": 95
    },
    "meta": {
      "observedAt": "2025-12-14T12:00:00Z",
      "direct": false,
      "rssi": -65,
      "gatewayId": "30020011223344550096"
    }
  }'
```

### is02データ確認

```bash
# is02からのイベントのみ抽出
curl -s http://192.168.96.201:8080/api/events?limit=2000 | \
  python3 -c "import sys, json; events = json.load(sys.stdin); \
  is02_events = [e for e in events if e.get('src') == 'is02']; \
  print(f'is02 events: {len(is02_events)}'); \
  print(json.dumps(is02_events[:3], indent=2))"
```

## Documentation References

必須参照ドキュメント:
- `docs/common/dataflow_specs.md`: デバイス間通信のデータ形式
- `docs/common/external_systems.md`: mobes2.0連携、Arduino-MCP使用方法
- `docs/common/直近の方針.md`: 離島運用前提の実装仕様、デバイス別要件
- `docs/is03/dev_prompt.md`: is03セットアップ手順
- `code/Orangepi/is03-relay/README.md`: is03 API仕様、systemd管理
- `code/ESP32/is02/README.md`: is02仕様書（バッチ送信・Web UI）
- `code/ESP32/is01/README.md`: is01仕様書（BLE広告・DeepSleep）
- `code/ESP32/global/README.md`: 共通モジュール概要

## Troubleshooting

### ESP32デバイスが起動しない

1. **シリアルモニタで確認**: `mcp__arduino_monitor_start(port: "COM3")` でログを確認
2. **I2C初期化失敗（is01）**: GPIO5がHIGHになっているか確認、DeepSleep復帰タイミングの問題の可能性
3. **WiFi接続失敗**: cluster1-6のいずれかが有効か確認、NVSに保存されたSSID/パスワードをチェック
4. **CIC取得失敗**: araneaDeviceGate URLとテナント認証情報を確認、NTP時刻同期が完了しているか確認

### is03にデータが届かない

1. **is02のOLED表示を確認**: 送信エラーが出ていないか
2. **is03のログ確認**: `wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201 'sudo journalctl -u is03-relay -n 50'`
3. **ネットワーク疎通確認**: is02から `curl http://192.168.96.201:8080/api/status` が成功するか
4. **リレーURL確認**: is02のNVS設定（`relay_pri`, `relay_sec`）が正しいか

### OTA更新が失敗する

1. **デバイスのIP確認**: OLEDまたはシリアルログでIPアドレスを確認
2. **WiFi接続確認**: `ping <デバイスIP>` で疎通確認
3. **ポート3232が開いているか**: ファイアウォール設定を確認
4. **Arduino OTAが有効か**: is02/is04/is05のコードで `OtaManager::begin()` が呼ばれているか確認

### is03のWebSocketが切断される

1. **ブラウザコンソール確認**: 自動再接続が動作しているか確認（index.html）
2. **is03サービス再起動**: `wsl sshpass -p 'isms12345@' ssh isms@192.168.96.201 'echo isms12345@ | sudo -S systemctl restart is03-relay'`
3. **メモリ不足**: `free -h` でメモリ使用量を確認、必要に応じてis03を再起動

## External Systems

- **mobes2.0**: https://github.com/warusakudeveroper/mobes2.0 - Firebase/Firestoreベースの IoT 統合管理システム
- **Arduino-MCP**: https://github.com/warusakudeveroper/Arduino-MCP - ESP32開発用MCPサーバー（自動セットアップ、コンパイル/アップロード、シリアル監視）
