# !!!!! 必読 !!!!! ESP32 モジュール役割分担ガイド

## ディレクトリ構成

```
code/ESP32/
├── global/                 ← 共通モジュール（全デバイスで再利用）
│   └── src/
│       ├── AraneaRegister.h/cpp      - デバイス登録（CIC取得）
│       ├── AraneaWebUI.h/cpp         - Web UI基底クラス
│       ├── DisplayManager.h/cpp      - I2C OLED表示
│       ├── HttpOtaManager.h/cpp      - HTTP経由OTA
│       ├── LacisIDGenerator.h/cpp    - LacisID生成
│       ├── MqttManager.h/cpp         - MQTT WebSocket接続
│       ├── NtpManager.h/cpp          - NTP時刻同期
│       ├── Operator.h/cpp            - 状態機械・競合制御
│       ├── SettingManager.h/cpp      - NVS永続化
│       ├── SshClient.h/cpp           - SSH接続
│       ├── StateReporter.h/cpp       - HTTP POST送信
│       └── WiFiManager.h/cpp         - WiFi接続管理
│
└── is10/                   ← IS10固有モジュール
    ├── is10.ino            - メインスケッチ（300行程度）
    ├── AraneaGlobalImporter.h  - 共通モジュール選択的インポート
    ├── AraneaSettings.h/cpp    - 施設設定（大量生産用）
    ├── HttpManagerIs10.h/cpp   - Web UI（is10専用）
    ├── RouterTypes.h           - ルーター設定構造体
    ├── SshPollerIs10.h/cpp     - SSHルーター巡回
    ├── StateReporterIs10.h/cpp - deviceStateReportペイロード構築
    ├── CelestialSenderIs10.h/cpp - CelestialGlobe SSOT送信
    └── MqttConfigHandler.h/cpp - MQTT config受信・適用
```

## 役割分担の原則

### global/ に置くもの（共通モジュール）

1. **複数デバイスで再利用される機能**
   - WiFi接続、NTP同期、OLED表示など
   - 例: WiFiManager は is10, is04, is02 すべてで使用

2. **送信処理（トランスポート層）**
   - HTTP POST送信、MQTT接続管理など
   - ペイロードの「中身」は知らない
   - 例: StateReporter.sendReport(jsonPayload) - JSONを受け取って送るだけ

3. **プラットフォーム共通の処理**
   - NVS読み書き、SPIFFS管理など

### is10/ に置くもの（デバイス固有モジュール）

1. **デバイス固有のビジネスロジック**
   - is10 = ルーター監視デバイス → SSH巡回
   - is04 = 接点出力デバイス → GPIO制御

2. **ペイロード構築（アプリケーション層）**
   - deviceStateReport の JSON構造は is10 固有
   - 例: StateReporterIs10.buildPayload() - ルーター情報を含むJSON生成

3. **Web UI 実装**
   - 基底クラス(AraneaWebUI)は global
   - 具体的なAPI実装(HttpManagerIs10)は is10

## 設計パターン: 送信機能と送信内容の分離

```
┌─────────────────────────────────────────────────────────────┐
│                     is10.ino (メイン)                       │
│  - 各モジュールの初期化 (begin)                              │
│  - ループ処理 (handle)                                       │
│  - コールバック設定                                          │
└─────────────────────────────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│StateReporterIs10│   │SshPollerIs10  │    │MqttConfigHandler│
│(ペイロード構築) │   │(SSH巡回制御)  │    │(設定受信・適用) │
│is10固有        │   │is10固有        │    │is10固有        │
└───────┬───────┘    └───────────────┘    └───────────────┘
        │
        ▼ JSON文字列を渡す
┌───────────────┐
│ StateReporter │  ← global（送信機能のみ）
│(HTTP POST送信)│
└───────────────┘
```

## ビルド設定

### 必須: パーティションスキーム

```
esp32:esp32:esp32:PartitionScheme=min_spiffs
```

**絶対禁止**: `huge_app` - OTA更新が不可能になる

### ライブラリパス

Arduino IDEの場合:
1. `/private/tmp/AraneaGlobalGeneric` を作成
2. `code/ESP32/global/src/` の内容をコピー
3. `library.properties` を作成

## 新しいデバイス追加時のガイドライン

1. **既存 global モジュールを最大限活用**
   - AraneaGlobalImporter.h を参照

2. **デバイス固有ディレクトリを作成**
   - 例: `code/ESP32/is04/`

3. **ペイロード構築は固有モジュールで**
   - 例: `StateReporterIs04.h/cpp`

4. **送信処理は global を使用**
   - 例: `StateReporter.sendReport(is04Payload)`

## 参照

- `is10/AraneaGlobalImporter.h` - モジュール一覧と使用方法
- `is10/AraneaSettings.h` - 大量生産設定の説明
- `docs/is10/` - is10固有のドキュメント
