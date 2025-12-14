# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ISMS（情報セキュリティマネジメントシステム）向けIoTデバイス開発プロジェクト。離島・VPN運用を前提とした温湿度監視・接点制御システム。

## System Architecture

### Device Roles (is01〜is05)

- **is01**: 電池式温湿度センサー。BLEアドバタイズで送信のみ（受信しない）。DeepSleep運用。OTA不可で固定。
- **is02**: 常時電源ゲートウェイ。is01のBLEを受信し、Wi-Fi経由でZero3/クラウドへ中継。
- **is03**: Orange Pi Zero3。is01のBLEを受信してローカル保存・ブラウザ表示。冗長化のため2台構成。
  - is03-1: `192.168.96.201:8080`
  - is03-2: `192.168.96.202:8080`
- **is04**: 接点出力デバイス（GPIO12/14制御）。ローカルMQTT/HTTP経由で制御。
- **is05**: 8chリード入力。窓/ドアの開閉状態を検知してZero3へ送信。

### Data Flow

```
is01 (BLE Advertise) → is03 (ローカル受信・表示)
                     → is02 (Wi-Fi中継) → クラウド（将来）
is04/is05 ←→ Zero3 (ローカルMQTT/HTTP)
```

### LacisID Format

`[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁]` = 20文字
例: `3` + `001` + `AABBCCDDEEFF` + `0001`

## ESP32 Development

### MCP Server

`mcp-arduino-esp32` が設定済み。Arduino CLIでのビルド・書き込みに使用。

### Common Modules (firmware/common/)

| Module | is01 | is02/04/05 | Purpose |
|--------|------|------------|---------|
| lacisIDGenerator | ○ | ○ | STA MACからlacisID生成 |
| araneaRegister | △(初回のみ) | ○ | cic_code取得・保存 |
| wifiManager | △(初回のみ) | ○ | cluster1-6への接続試行 |
| displayManager | ○ | ○ | I2C OLED 128x64表示 |
| ntpManager | × | ○ | 時刻同期 |
| settingManager | ○(NVS) | ○(SPIFFS) | 設定永続化 |
| HTTPManager | × | ○ | Web UI・API |
| otaManager | × | ○ | OTA更新 |
| Operator | ○ | ○ | 状態機械・競合制御 |

### is01 Critical Notes

- I2C初期化でコケやすい。**I2C処理は必ず直列実行**（並列禁止）
- DeepSleep復帰→I2C初期化の順序がシビア
- CIC未取得でもBLE広告は必ず出す（Zero3で検知可能にする）

### Default WiFi

SSID: `cluster1`〜`cluster6`, Password: `isms12345@`

### Relay Endpoints (Zero3)

- primary: `192.168.96.201`
- secondary: `192.168.96.202`

## Orange Pi (is03) Development

### OS

Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img

### Setup User

User: `isms`, Password: `isms12345@`

### Service

```bash
# アプリ状態確認
sudo systemctl status is03-relay

# API確認
curl http://localhost:8080/api/status
curl http://localhost:8080/api/events?limit=5
```

### Data Storage

- メモリ: 最新2000件保持（画面表示用）
- SQLite: `/var/lib/is03-relay/relay.sqlite`（バッチ書き込み）
- テナント（市山水産株式会社）
TENANT_NAME="市山水産株式会社"
TID="T2025120608261484221"
FID="9000"

# テナントプライマリ（perm61）\
lacisID=12767487939173857894
TENANT_PRIMARY_EMAIL="info+ichiyama@neki.tech"
TENANT_PRIMARY_CIC="263238"
TENANT_PRIMARY_PASS="dJBU^TpG%j$5"\
\
これは今回のプロジェクトでの共通事項なので/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/docs/commonにもこの内容を記載したドキュメントを作成