# Aranea Device Catalog

**更新日**: 2025/12/22
**目的**: 全デバイスの一覧と共通設計原則

---

## 1. デバイス一覧

### 1.1 センサー系

| 製品コード | 名称 | 正式名称 | 概要 |
|-----------|------|---------|------|
| AR-IS01A | is01a | Temp&HumSensor (Cluster Node) | 電池式温湿度BLEセンサー |
| AR-IS02A | is02a | Temp&HumSensor (Master) | BLE受信マスター＋温湿度センサー |
| AR-IS05A | is05a | 8ch Detector | 8ch入力検出器（Webhook対応） |

### 1.2 コントローラー系

| 製品コード | 名称 | 正式名称 | 概要 |
|-----------|------|---------|------|
| AR-IS04A | is04a | Window & Door Controller | 2ch接点出力（物理スイッチ付き） |
| AR-IS06A | is06a | 6ch Relay Controller | 6chトリガー出力（PWM対応） |

### 1.3 ネットワーク監視系

| 製品コード | 名称 | 正式名称 | 概要 |
|-----------|------|---------|------|
| AR-IS10 | is10 | SSH Poller | ASUSWRT/OpenWrt SSHポーラー |
| AR-IS10M | is10m | Mercury AC Poller | Mercury AC WiFiコントローラーポーラー |
| AR-IS10D | is10D | ARP Scanner | サブネットARPスキャナー |

### 1.4 サーバー系

| 製品コード | 名称 | 正式名称 | プラットフォーム |
|-----------|------|---------|----------------|
| AR-IS03A | is03a | LocalMQTT-Server | Orange Pi Zero3 |

---

## 2. 共通設計原則

### 2.1 必須コンポーネント

全デバイスで以下は**必須**:

| コンポーネント | 役割 |
|---------------|------|
| LacisIDGenerator | lacisID生成（IoT動作に必須） |
| AraneaRegister | AraneaDeviceGateでのCIC取得（IoT動作に必須） |
| SettingManager | NVS永続化 |
| WiFiManager | WiFi接続・APモード切替 |

### 2.2 ESP32設計原則

```
重要: ESP32では以下を遵守

1. セマフォとWDTの過剰制御を避ける
   - 制御と監査によって不安定化する
   - 余計なことはなるべくやらない

2. 本体プログラムのタイミングとメモリ管理を重視
   - delay()の適切な使用
   - ヒープの断片化防止

3. コードのシンプル化
   - 過剰なエラーハンドリングを避ける
   - 複雑な状態機械を避ける

4. 可能な限りシングルタスクで実装
   - xTaskCreate()の使用を最小限に
   - loop()ベースの設計を優先

5. パーティション: min_SPIFFS使用
   - huge_appは絶対禁止（OTA不可になる）
   - OTA領域を確保
```

### 2.3 起動フロー（共通）

```
1. WiFi設定確認
   └─ 空の場合 → APモード起動（Web UI）
   └─ 設定済み → WiFi接続

2. WiFi接続

3. LacisID生成（LacisIDGenerator）【必須】

4. AraneaDeviceGate登録（AraneaRegister）【必須】
   └─ CIC取得・NVS保存

5. 各デバイス固有の初期化

6. 通常動作モード
```

---

## 3. 共通モジュール

### 3.1 global/src/ モジュール

| モジュール | 用途 | 必須 |
|-----------|------|------|
| WiFiManager | WiFi接続・APモード | ○ |
| SettingManager | NVS永続化 | ○ |
| DisplayManager | I2C OLED表示 | △ |
| NtpManager | NTP時刻同期 | △ |
| MqttManager | MQTT接続 | △ |
| LacisIDGenerator | lacisID生成 | **必須** |
| AraneaRegister | CIC取得 | **必須** |
| AraneaWebUI | Web UI基底クラス | △ |
| HttpOtaManager | OTA更新 | △ |
| Operator | 状態機械 | △ |
| IOController | I/O制御（新規） | is04a/05a/06a |
| PWMController | PWM制御（新規） | is06a |

### 3.2 デバイス固有モジュール

各デバイスの固有モジュールは `code/ESP32/{device}/` に配置。

---

## 4. ディレクトリ構成

```
code/ESP32/
├── global/
│   ├── src/               # 共通モジュール
│   └── design/            # 共通モジュール仕様書
│
├── is01a/
│   ├── is01a.ino          # メインスケッチ
│   ├── *Manager.h/cpp     # 固有モジュール
│   └── design/            # 設計書
│
├── is02a/
├── is03a/                  # Orange Pi用
├── is04a/
├── is05a/
├── is06a/
├── is10/
├── is10m/
└── is10D/
```

---

## 5. 設計書一覧

| デバイス | 設計書 |
|---------|--------|
| is01a | `code/ESP32/is01a/design/DESIGN.md` |
| is02a | `code/ESP32/is02a/design/DESIGN.md` |
| is03a | `code/ESP32/is03a/design/DESIGN.md` |
| is04a | `code/ESP32/is04a/design/DESIGN.md` |
| is05a | `code/ESP32/is05a/design/DESIGN.md` |
| is06a | `code/ESP32/is06a/design/DESIGN.md` |
| is10D | `code/ESP32/is10D/design/DESIGN.md` |
| IOController | `code/ESP32/global/design/IOController_SPEC.md` |
| PWMController | `code/ESP32/global/design/PWMController_SPEC.md` |

---

## 6. 参照

- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
- **ISMS版アーカイブ**: `archive_ISMS/ESP32/`
