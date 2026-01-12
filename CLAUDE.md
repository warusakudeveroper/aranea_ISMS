CLAUDE.md

  

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

  

## important!!!
↓↓
Please be sure to read, refer to and comply with this.↓↓
`/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/The_golden_rules.md`
  
  

## Project Overview

  

araneaシリーズIoTデバイス開発(araneaDevices)プロジェクト。離島・VPN運用を前提とした温湿度監視・接点制御システム。ISMSシリーズは特定プロジェクトに帰属するプロトタイプの位置付けです。汎用一般ラインはaranea isシリーズです

  

## System Architecture

  

### Device Roles (is01〜is05)

  

- **is01**: 電池式温湿度センサー。BLEアドバタイズで送信のみ（受信しない）。DeepSleep運用。OTA不可で固定。

- **is02**: 常時電源ゲートウェイ。is01のBLEを受信し、Wi-Fi経由でZero3/クラウドへ中継。

- **is03**: Orange Pi Zero3。is01のBLEを受信してローカル保存・ブラウザ表示。冗長化のため2台構成。

- is03-1: `192.168.96.201:8080`

- is03-2: `192.168.96.202:8080`

- **is04**: 接点出力デバイス（GPIO12/14制御）。ローカルMQTT/HTTP経由で制御。

- **is05**: 8chリード入力。窓/ドアの開閉状態を検知してZero3へ送信。

- **is10**: AsusWart,OpenWart SSH Status取得

- **is20s**: wireSharkトラフィックアナライザー

- テスト環境サーバー1 is20s-tam:`http://192.168.125.248:8080/`

- テスト環境サーバー2 is20s-to:`http://192.168.126.249:8080/`

- テスト環境サーバー3 is20s-akb:`http://192.168.3.250:8080/`

- SSH設定(共通)

- ユーザー: mijeosadmin / パスワード: mijeos12345@

- sudoグループに追加済み

- SSHサービス: enabled（自動起動設定済み）

- IPアドレス: DHCPのまま（固定なし）

  

- **is21**: 画像解析推論サーバー
	  IP: 192.168.3.240
	  Username :mijeosadmin
	  Password:mijeos12345@(sudo可能)
	  OS:Armbian 25.11.2 noble

- **is22**: RTSPカメラ総合管理サーバー
	  IP: 192.168.125.246
	  Username :mijeosadmin
	  Password:mijeos12345@(sudo可能)

## Build Rules（重要）

### is22 Rustビルド
- **Cargoビルドは必ずサーバー側（192.168.125.246）で実行すること**
- ローカルでのクロスコンパイルは禁止
- ソースコードをサーバーに転送後、サーバー上で`cargo build --release`を実行
- ビルドパス: `/opt/is22/`
- バイナリ: `/opt/is22/target/release/camserver`
- サービス再起動: `sudo systemctl restart is22`

### Data Flow

  

```

is01 (BLE Advertise) → is03 (ローカル受信・表示)

→ is02 (Wi-Fi中継) → クラウド（将来）

is04/is05 ←→ Zero3 (ローカルMQTT/HTTP)

```

  

### LacisID Format

  

`[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁]` = 20文字
araneaSDKのナレッジおよびMetatronで確認

  

## ESP32 Development

  

### MCP Server

  

`mcp-arduino-esp32` が設定済み。Arduino CLIでのビルド・書き込みに使用。

### ESP32 Partition Scheme（重要）

  

- **`huge_app`は絶対に使用禁止** - OTA領域がなくなりリモート更新不可になる

- フラッシュ容量不足時は **`min_spiffs`** を使用すること（OTA領域を維持）

- 例: `esp32:esp32:esp32:PartitionScheme=min_spiffs`

  
### is01 Critical Notes

  

- I2C初期化でコケやすい。**I2C処理は必ず直列実行**（並列禁止）

- DeepSleep復帰→I2C初期化の順序がシビア

- CIC未取得でもBLE広告は必ず出す（Zero3で検知可能にする）

  

### Default WiFi

  

SSID: `cluster1`〜`cluster6`, Password: `isms12345@`

# テナントアカウント情報

市山水産株式会社　tid T2025120608261484221 fid9000(市山水産(牛深))=192.168.96.0/23

テナントプライマリ:info+ichiyama@neki.tech

テナントプライマリlacisID:12767487939173857894

テナントプライマリCIC:263238

  

mijeo.inc tid T2025120621041161827

テナントプライマリ:soejim@mijeos.com

テナントプライマリlacisID:18217487937895888001

テナントプライマリCIC:204965

fid:0150 HALE京都丹波口ホテル=192.168.125.0/24

fid:0151 HALE京都東寺ホテル=192.168.126.0/24

  

このデバイス(サーバー、AIサーバー)の登録を行うアカウント

mijeo.inc tid T2025120621041161827

テナントプライマリ:soejim@mijeos.com

テナントプライマリlacisID:18217487937895888001

テナントプライマリCIC:204965