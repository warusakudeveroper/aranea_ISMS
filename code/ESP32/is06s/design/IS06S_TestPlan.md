# IS06S テスト計画

**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**最終更新**: 2025/01/23
**バージョン**: 1.2

---

## 1. テスト概要

### 1.1 テスト方針

The_golden_rules.md原則9に基づき、以下のテストを必須とする：
- **バックエンドテスト**: ESP32ファームウェア機能テスト
- **フロントエンドテスト**: Web UI機能テスト
- **実UI/UXテスト**: Chromeブラウザでの実機操作テスト

### 1.2 テスト環境

| 項目 | 値 |
|------|-----|
| SSID | sorapia_facility_wifi |
| Password | JYuDzjhi |
| Subnet | 192.168.77.0/24 |
| Gateway | 192.168.77.1 |
| TID | T2025120621041161827 |
| テナントプライマリEmail | soejim@mijeos.com |
| テナントプライマリCIC | 204965 |
| テナントプライマリLacisID | 18217487937895888001 |

### 1.3 テストツール

| ツール | 用途 |
|--------|------|
| Arduino Serial Monitor | シリアル出力確認 |
| curl / Postman | HTTP API テスト |
| Chrome DevTools | Web UI デバッグ |
| MQTT Explorer | MQTT 通信確認 |
| マルチメーター | GPIO電圧確認 |
| オシロスコープ | PWM波形確認 |

---

## 2. 単体テスト

### 2.1 GPIO テスト

#### 2.1.1 Digital Output テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-GPIO-01 | CH1 HIGH出力 | `digitalWrite(18, HIGH)` | GPIO18が3.3V | ⬜ |
| UT-GPIO-02 | CH1 LOW出力 | `digitalWrite(18, LOW)` | GPIO18が0V | ⬜ |
| UT-GPIO-03 | CH2 HIGH出力 | `digitalWrite(5, HIGH)` | GPIO5が3.3V | ⬜ |
| UT-GPIO-04 | CH2 LOW出力 | `digitalWrite(5, LOW)` | GPIO5が0V | ⬜ |
| UT-GPIO-05 | CH3 HIGH出力 | `digitalWrite(15, HIGH)` | GPIO15が3.3V | ⬜ |
| UT-GPIO-06 | CH3 LOW出力 | `digitalWrite(15, LOW)` | GPIO15が0V | ⬜ |
| UT-GPIO-07 | CH4 HIGH出力 | `digitalWrite(27, HIGH)` | GPIO27が3.3V | ⬜ |
| UT-GPIO-08 | CH4 LOW出力 | `digitalWrite(27, LOW)` | GPIO27が0V | ⬜ |

#### 2.1.2 PWM Output テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-PWM-01 | CH1 PWM 0% | ledcWrite(0, 0) | デューティ0% | ⬜ |
| UT-PWM-02 | CH1 PWM 50% | ledcWrite(0, 128) | デューティ50% | ⬜ |
| UT-PWM-03 | CH1 PWM 100% | ledcWrite(0, 255) | デューティ100% | ⬜ |
| UT-PWM-04 | PWM周波数確認 | オシロスコープ | 5kHz | ⬜ |

#### 2.1.3 Digital Input テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-DI-01 | CH5 LOW検知 | GPIO14オープン | digitalRead=0 | ⬜ |
| UT-DI-02 | CH5 HIGH検知 | GPIO14に3.3V印加 | digitalRead=1 | ⬜ |
| UT-DI-03 | CH6 LOW検知 | GPIO12オープン | digitalRead=0 | ⬜ |
| UT-DI-04 | CH6 HIGH検知 | GPIO12に3.3V印加 | digitalRead=1 | ⬜ |
| UT-DI-05 | デバウンス確認 | 高速ON/OFF | 3000ms以内無視 | ⬜ |

#### 2.1.4 System PIN テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-SYS-01 | Reconnect入力検知 | GPIO25に3.3V | 入力検知 | ⬜ |
| UT-SYS-02 | Reset入力検知 | GPIO26に3.3V | 入力検知 | ⬜ |

---

### 2.2 Is06PinManager テスト

#### 2.2.1 Digital Output (Momentary)

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-PM-01 | パルス開始 | setPinState("CH1","on") | state="on", GPIO18=HIGH | ⬜ |
| UT-PM-02 | パルス終了 | validity ms後 | state="off", GPIO18=LOW | ⬜ |
| UT-PM-03 | validity override | setPinState("CH1","on",5000) | 5000ms後にOFF | ⬜ |
| UT-PM-04 | コールバック呼出 | 状態変化時 | callback実行 | ⬜ |

#### 2.2.2 Digital Output (Alternate)

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-PM-05 | OFF→ON | setPinState("CH1","on") | state="on" | ⬜ |
| UT-PM-06 | ON→OFF | setPinState("CH1","off") | state="off" | ⬜ |
| UT-PM-07 | 状態保持 | 再起動 | NVS復元 | ⬜ |

#### 2.2.3 PWM Output

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-PM-08 | PWM設定 | setPinState("CH1","50") | PWM 50% | ⬜ |
| UT-PM-09 | Slow変化 | 0→100 | rateOfChange ms | ⬜ |
| UT-PM-10 | Rapid変化 | 0→100 | 即座に変化 | ⬜ |

#### 2.2.4 Digital Input (連動)

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-PM-11 | 単一連動 | CH5入力 | CH1トリガー | ⬜ |
| UT-PM-12 | 複数連動 | CH5入力(allocation=[CH1,CH2]) | CH1,CH2トリガー | ⬜ |
| UT-PM-13 | rotate動作 | CH5入力(PWM) | 0→30→60→100→0 | ⬜ |

#### 2.2.5 PIN Enable/Disable テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-EN-01 | enabled=false時API拒否 | CH1 enabled=false, POST /api/pin/CH1/state | 403 + エラーメッセージ | ⬜ |
| UT-EN-02 | enabled=false時MQTT拒否 | CH1 enabled=false, MQTTコマンド送信 | ACK error="PIN disabled" | ⬜ |
| UT-EN-03 | enabled=false時UI非表示 | CH1 enabled=false | ボタン/スライダーdisabled | ⬜ |
| UT-EN-04 | enabled=true復帰 | enabled=false→true変更 | 操作可能に復帰 | ⬜ |
| UT-EN-05 | enabled設定NVS保存 | enabled変更→再起動 | 設定維持 | ⬜ |
| UT-EN-06 | Input連動enabled=false | CH5→CH1, CH1 enabled=false | 連動無効 | ⬜ |

#### 2.2.6 デフォルト解決チェーン テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-DEF-01 | PINSettings優先 | CH1 defaultValidity=5000, PINglobal=3000 | 5000ms適用 | ⬜ |
| UT-DEF-02 | PINglobalフォールバック | CH1 defaultValidity空, PINglobal=3000 | 3000ms適用 | ⬜ |
| UT-DEF-03 | AraneaSettingsDefaultsフォールバック | PINSettings空, PINglobal空 | AraneaSettingsDefaults値適用 | ⬜ |
| UT-DEF-04 | Digital actionMode解決 | CH1 actionMode空, PINglobal.Digital.defaultActionMode="Alt" | Alt適用 | ⬜ |
| UT-DEF-05 | PWM RateOfChange解決 | CH1 RateOfChange空, PINglobal.PWM.defaultRateOfChange=4000 | 4000ms適用 | ⬜ |
| UT-DEF-06 | expiry解決 | CH1 defaultexpiry空, PINglobal.common.defaultexpiry=1 | 1day適用 | ⬜ |

---

### 2.3 WiFi テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-WIFI-01 | SSID1接続 | 設定SSID | 接続成功 | ⬜ |
| UT-WIFI-02 | フォールバック | SSID1失敗 | SSID2試行 | ⬜ |
| UT-WIFI-03 | APモード移行 | 全SSID失敗 | APモード起動 | ⬜ |
| UT-WIFI-04 | 再接続 | WiFi切断 | 自動再接続 | ⬜ |

---

### 2.4 NVS テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-NVS-01 | 文字列保存 | setString("key","value") | 保存成功 | ⬜ |
| UT-NVS-02 | 文字列読込 | getString("key") | "value"取得 | ⬜ |
| UT-NVS-03 | 整数保存 | setInt("key",123) | 保存成功 | ⬜ |
| UT-NVS-04 | 整数読込 | getInt("key") | 123取得 | ⬜ |
| UT-NVS-05 | デフォルト値 | getString("nokey","def") | "def"取得 | ⬜ |

#### 2.4.2 NVS PINglobal初期化テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| UT-NVS-06 | PINglobal初期書込 | ファクトリーリセット→起動 | dig_validity=3000, dig_debounce=3000等 | ⬜ |
| UT-NVS-07 | PINglobal読込 | 起動時 | PINglobal構造体に正しくロード | ⬜ |
| UT-NVS-08 | PINglobal更新 | API経由でPINglobal変更 | NVS保存成功 | ⬜ |
| UT-NVS-09 | PINglobal再起動復元 | PINglobal変更→再起動 | 変更値維持 | ⬜ |
| UT-NVS-10 | enabled初期値 | ch1_enabled読込(未設定) | true（デフォルト） | ⬜ |
| UT-NVS-11 | enabled保存/復元 | ch1_enabled=false→再起動 | false維持 | ⬜ |

---

## 3. API テスト（バックエンド）

### 3.1 HTTP API テスト

#### 3.1.1 状態取得API

| ID | テスト項目 | リクエスト | 期待結果 | 結果 |
|----|-----------|-----------|----------|------|
| AT-API-01 | 全状態取得 | GET /api/status | 200 + JSON | ⬜ |
| AT-API-02 | CH1状態取得 | GET /api/pin/CH1/state | 200 + JSON | ⬜ |
| AT-API-03 | 無効CH | GET /api/pin/CH9/state | 400 | ⬜ |

#### 3.1.2 状態変更API

| ID | テスト項目 | リクエスト | 期待結果 | 結果 |
|----|-----------|-----------|----------|------|
| AT-API-04 | PIN ON | POST /api/pin/CH1/state {"state":"on"} | 200 | ⬜ |
| AT-API-05 | PIN OFF | POST /api/pin/CH1/state {"state":"off"} | 200 | ⬜ |
| AT-API-06 | PWM設定 | POST /api/pin/CH1/state {"state":"50"} | 200 | ⬜ |
| AT-API-07 | validity指定 | POST {"state":"on","validity":5000} | 200 | ⬜ |
| AT-API-08 | 不正JSON | POST /api/pin/CH1/state {invalid} | 400 | ⬜ |

#### 3.1.3 設定API

| ID | テスト項目 | リクエスト | 期待結果 | 結果 |
|----|-----------|-----------|----------|------|
| AT-API-09 | PIN設定取得 | GET /api/pin/CH1/setting | 200 + JSON | ⬜ |
| AT-API-10 | PIN設定変更 | POST /api/pin/CH1/setting | 200 | ⬜ |
| AT-API-11 | globalSettings取得 | GET /api/settings | 200 + JSON | ⬜ |
| AT-API-12 | globalSettings変更 | POST /api/settings | 200 | ⬜ |

### 3.2 MQTT テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| AT-MQTT-01 | 接続 | ブローカー接続 | 接続成功 | ⬜ |
| AT-MQTT-02 | Subscribe | command topic | Subscribe成功 | ⬜ |
| AT-MQTT-03 | コマンド受信 | JSON送信 | PIN制御実行 | ⬜ |
| AT-MQTT-04 | ACK送信 | コマンド後 | ACK Publish | ⬜ |
| AT-MQTT-05 | 状態Publish | 状態変化時 | state Publish | ⬜ |

### 3.3 状態送信テスト

| ID | テスト項目 | 手順 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| AT-SR-01 | ペイロード構築 | buildPayload() | JSON正常 | ⬜ |
| AT-SR-02 | HTTP送信 | sendReport() | 200応答 | ⬜ |
| AT-SR-03 | 認証情報 | auth部確認 | TID/CIC正常 | ⬜ |

---

## 4. フロントエンドテスト（Web UI）

### 4.1 ページ表示テスト

| ID | テスト項目 | URL | 期待結果 | 結果 |
|----|-----------|-----|----------|------|
| FT-PAGE-01 | トップページ | / | 200 | ⬜ |
| FT-PAGE-02 | PIN操作ページ | /pincontrol | 200 | ⬜ |
| FT-PAGE-03 | PIN設定ページ | /pinsetting | 200 | ⬜ |
| FT-PAGE-04 | ネットワーク設定 | /network | 200 | ⬜ |
| FT-PAGE-05 | システム設定 | /system | 200 | ⬜ |

### 4.2 PIN操作UIテスト

| ID | テスト項目 | 操作 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| FT-PIN-01 | 状態表示 | ページ読込 | 全PIN状態表示 | ⬜ |
| FT-PIN-02 | ON操作 | ONボタンクリック | state=on | ⬜ |
| FT-PIN-03 | OFF操作 | OFFボタンクリック | state=off | ⬜ |
| FT-PIN-04 | PWMスライダー | スライダー操作 | PWM値反映 | ⬜ |
| FT-PIN-05 | 自動更新 | 状態変化 | 画面更新 | ⬜ |

### 4.3 PIN設定UIテスト

| ID | テスト項目 | 操作 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| FT-SET-01 | 設定表示 | ページ読込 | 現在設定表示 | ⬜ |
| FT-SET-02 | type変更 | ドロップダウン | type変更反映 | ⬜ |
| FT-SET-03 | name変更 | テキスト入力 | name変更反映 | ⬜ |
| FT-SET-04 | actionMode変更 | ドロップダウン | mode変更反映 | ⬜ |
| FT-SET-05 | 保存 | 保存ボタン | NVS保存 | ⬜ |

---

## 5. 実UI/UXテスト（Chrome）

### 5.1 テスト環境

- ブラウザ: Chrome最新版
- デバイス: Windows / Mac / モバイル
- ネットワーク: テスト環境WiFi（192.168.77.0/24）

### 5.2 シナリオテスト

#### 5.2.1 初期セットアップシナリオ

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-01-01 | 電源投入 | ESP32電源ON | APモード起動 | ⬜ |
| ST-01-02 | AP接続 | IS06S_SETUPに接続 | 接続成功 | ⬜ |
| ST-01-03 | Web UI | 192.168.250.1アクセス | ページ表示 | ⬜ |
| ST-01-04 | WiFi設定 | SSID/PASS入力 | 設定保存 | ⬜ |
| ST-01-05 | 再起動 | リブート | STAモード接続 | ⬜ |
| ST-01-06 | 動作確認 | 新IPアクセス | ページ表示 | ⬜ |

#### 5.2.2 PIN操作シナリオ（Digital Momentary）

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-02-01 | ページ表示 | /pincontrol | CH1-6表示 | ⬜ |
| ST-02-02 | CH1 ON | ONボタン | LED点灯 | ⬜ |
| ST-02-03 | 自動OFF | validity後 | LED消灯 | ⬜ |
| ST-02-04 | 状態更新 | UI確認 | state=off表示 | ⬜ |

#### 5.2.3 PIN操作シナリオ（PWM調光）

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-03-01 | PWM設定 | /pinsetting | type=pwmOutput | ⬜ |
| ST-03-02 | 操作画面 | /pincontrol | スライダー表示 | ⬜ |
| ST-03-03 | 50%設定 | スライダー50% | LED半輝度 | ⬜ |
| ST-03-04 | Slow変化 | 0→100 | 徐々に明るく | ⬜ |

#### 5.2.4 物理スイッチ連動シナリオ

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-04-01 | Input設定 | CH5 digitalInput | 設定保存 | ⬜ |
| ST-04-02 | allocation | CH5→CH1 | 設定保存 | ⬜ |
| ST-04-03 | 物理入力 | GPIO14 HIGH | CH1トリガー | ⬜ |
| ST-04-04 | UI更新 | 画面確認 | CH1 state=on | ⬜ |

#### 5.2.5 MQTT連携シナリオ

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-05-01 | MQTT接続 | 設定確認 | 接続中表示 | ⬜ |
| ST-05-02 | コマンド送信 | MQTT Explorer | PIN動作 | ⬜ |
| ST-05-03 | ACK確認 | トピック確認 | ACK受信 | ⬜ |
| ST-05-04 | 状態確認 | state topic | 状態Publish | ⬜ |

#### 5.2.6 System PINシナリオ

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-06-01 | Reconnect | GPIO25 5秒長押し | WiFi再接続 | ⬜ |
| ST-06-02 | OLED表示 | カウントダウン | 表示確認 | ⬜ |
| ST-06-03 | Reset | GPIO26 15秒長押し | ファクトリーリセット | ⬜ |
| ST-06-04 | 初期化確認 | 再起動後 | APモード | ⬜ |
| ST-06-05 | Reconnect後NTP同期 | Reconnect実行後 | NTP再同期成功 | ⬜ |
| ST-06-06 | Reconnect後expiryDate検証 | NTP同期後 | 現在時刻でexpiryDate判定正常 | ⬜ |

#### 5.2.7 OTAシナリオ

| ID | ステップ | 操作 | 期待結果 | 結果 |
|----|----------|------|----------|------|
| ST-07-01 | OTAページ | /system | OTA表示 | ⬜ |
| ST-07-02 | ファイル選択 | binファイル | 選択完了 | ⬜ |
| ST-07-03 | アップロード | 更新ボタン | 進捗表示 | ⬜ |
| ST-07-04 | 完了確認 | 再起動後 | 新版動作 | ⬜ |

---

## 6. 異常系テスト

### 6.1 通信異常

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-COM-01 | WiFi切断 | APオフ | 再接続試行 | ⬜ |
| ET-COM-02 | MQTT切断 | ブローカー停止 | 再接続試行 | ⬜ |
| ET-COM-03 | HTTP送信失敗 | サーバー停止 | リトライ+ログ | ⬜ |

### 6.2 入力異常

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-IN-01 | 不正チャンネル | CH9指定 | 400エラー | ⬜ |
| ET-IN-02 | 不正JSON | 壊れたJSON | 400エラー | ⬜ |
| ET-IN-03 | 範囲外PWM | state="150" | 100にクランプ | ⬜ |
| ET-IN-04 | allocation違反 | Digital+PWM混在 | 設定拒否 | ⬜ |

### 6.2.2 型不一致検証（userObjectDetail）

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-TYPE-01 | DHCP文字列型 | DHCP="true" (string) | 正常処理 | ⬜ |
| ET-TYPE-02 | DHCP boolean誤送信 | DHCP=true (boolean) | 400エラーまたは文字列変換 | ⬜ |
| ET-TYPE-03 | exclusiveConnection文字列型 | exclusiveConnection="false" | 正常処理 | ⬜ |
| ET-TYPE-04 | lacisOath文字列型 | lacisOath="false" | 正常処理 | ⬜ |
| ET-TYPE-05 | enabled文字列型 | enabled="true" | 正常処理 | ⬜ |
| ET-TYPE-06 | Staticキー大文字 | Static: {...} | 正常処理 | ⬜ |
| ET-TYPE-07 | staticキー小文字誤り | static: {...} | 400エラーまたは警告ログ | ⬜ |

### 6.3 システム異常

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-SYS-01 | WDTタイムアウト | 無限ループ | 自動リブート | ⬜ |
| ET-SYS-02 | ヒープ枯渇 | メモリリーク | 警告ログ | ⬜ |
| ET-SYS-03 | I2Cエラー | OLEDなし | 動作継続 | ⬜ |
| ET-SYS-04 | NVS書込失敗 | 容量超過 | エラーログ | ⬜ |

### 6.4 expiryDate・有効期限テスト

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-EXP-01 | expiryDate超過 | 現在時刻 > expiryDate | PIN操作無効化 | ⬜ |
| ET-EXP-02 | expiryDate警告 | 24時間以内に期限切れ | 警告ログ出力 | ⬜ |
| ET-EXP-03 | Validity > expiry | Validity超過 | expiry時間で制限 | ⬜ |
| ET-EXP-04 | expiryDate更新 | 新expiryDate設定 | 期限延長成功 | ⬜ |
| ET-EXP-05 | 期限切れ後操作 | expiryDate超過後 | エラー応答（403） | ⬜ |

### 6.5 AP・WiFi設定テスト

| ID | テスト項目 | 条件 | 期待結果 | 結果 |
|----|-----------|------|----------|------|
| ET-AP-01 | 全SSID空欄 | wifiSetting全空 | APモード起動 | ⬜ |
| ET-AP-02 | 全SSID接続失敗 | 全SSID到達不能 | APモード移行 | ⬜ |
| ET-AP-03 | SSID順次接続 | SSID1失敗、SSID2成功 | SSID2接続 | ⬜ |
| ET-AP-04 | SSID中間空欄 | SSID2空、SSID3設定 | SSID3試行 | ⬜ |
| ET-AP-05 | exclusiveConnection=true | 2クライアント接続試行 | 2台目拒否 | ⬜ |
| ET-AP-06 | exclusiveConnection=false | 2クライアント接続試行 | 2台目許可 | ⬜ |
| ET-AP-07 | APPASSオープン | APPASS="" | パスワードなし接続 | ⬜ |
| ET-AP-08 | APPASSあり | APPASS設定 | パスワード必須 | ⬜ |

---

## 7. 性能テスト

| ID | テスト項目 | 基準 | 結果 |
|----|-----------|------|------|
| PT-01 | 起動時間 | 10秒以内 | ⬜ |
| PT-02 | API応答時間 | 100ms以内 | ⬜ |
| PT-03 | PWM変化遅延 | 10ms以内 | ⬜ |
| PT-04 | Input検知遅延 | 20ms以内 | ⬜ |
| PT-05 | 状態送信遅延 | 500ms以内 | ⬜ |
| PT-06 | ヒープ使用量 | 100KB以下 | ⬜ |

---

## 8. テスト結果サマリ

### 8.1 単体テスト

| カテゴリ | 合計 | Pass | Fail | Skip |
|---------|------|------|------|------|
| GPIO | 12 | - | - | - |
| PinManager | 13 | - | - | - |
| PIN Enable/Disable | 6 | - | - | - |
| デフォルト解決チェーン | 6 | - | - | - |
| WiFi | 4 | - | - | - |
| NVS (基本) | 5 | - | - | - |
| NVS PINglobal初期化 | 6 | - | - | - |
| **小計** | **52** | **-** | **-** | **-** |

### 8.2 APIテスト

| カテゴリ | 合計 | Pass | Fail | Skip |
|---------|------|------|------|------|
| HTTP API | 12 | - | - | - |
| MQTT | 5 | - | - | - |
| 状態送信 | 3 | - | - | - |
| **小計** | **20** | **-** | **-** | **-** |

### 8.3 フロントエンドテスト

| カテゴリ | 合計 | Pass | Fail | Skip |
|---------|------|------|------|------|
| ページ表示 | 5 | - | - | - |
| PIN操作UI | 5 | - | - | - |
| PIN設定UI | 5 | - | - | - |
| **小計** | **15** | **-** | **-** | **-** |

### 8.4 実UI/UXテスト

| カテゴリ | 合計 | Pass | Fail | Skip |
|---------|------|------|------|------|
| 初期セットアップ | 6 | - | - | - |
| Digital操作 | 4 | - | - | - |
| PWM操作 | 4 | - | - | - |
| 物理スイッチ | 4 | - | - | - |
| MQTT連携 | 4 | - | - | - |
| System PIN | 6 | - | - | - |
| OTA | 4 | - | - | - |
| **小計** | **32** | **-** | **-** | **-** |

### 8.5 その他テスト

| カテゴリ | 合計 | Pass | Fail | Skip |
|---------|------|------|------|------|
| 通信異常 | 3 | - | - | - |
| 入力異常 | 4 | - | - | - |
| 型不一致検証 | 7 | - | - | - |
| システム異常 | 4 | - | - | - |
| expiryDate・有効期限 | 5 | - | - | - |
| AP・WiFi設定 | 8 | - | - | - |
| 性能 | 6 | - | - | - |
| **小計** | **37** | **-** | **-** | **-** |

### 8.6 総合

| 項目 | 値 |
|------|-----|
| 総テスト数 | 156 |
| Pass | - |
| Fail | - |
| Skip | - |
| Pass率 | -% |

**内訳**: 単体52 + API20 + フロントエンド15 + 実UI/UX32 + その他37 = 156

---

## 9. MECE確認

### 9.1 テスト網羅確認

| 機能領域 | テスト数 | 漏れ確認 |
|---------|---------|---------|
| GPIO | 12 | ✅ |
| PinManager | 13 | ✅ |
| PIN Enable/Disable | 6 | ✅ |
| デフォルト解決チェーン | 6 | ✅ |
| WiFi | 4 | ✅ |
| NVS (基本) | 5 | ✅ |
| NVS PINglobal初期化 | 6 | ✅ |
| HTTP API | 12 | ✅ |
| MQTT | 5 | ✅ |
| 状態送信 | 3 | ✅ |
| Web UI | 15 | ✅ |
| シナリオ | 32 | ✅ |
| 通信異常 | 3 | ✅ |
| 入力異常 | 4 | ✅ |
| 型不一致検証 | 7 | ✅ |
| システム異常 | 4 | ✅ |
| expiryDate・有効期限 | 5 | ✅ |
| AP・WiFi設定 | 8 | ✅ |
| 性能 | 6 | ✅ |

### 9.2 テストレベル確認

| レベル | 対応 |
|--------|------|
| 単体テスト | ✅ バックエンド |
| APIテスト | ✅ バックエンド |
| UIテスト | ✅ フロントエンド |
| シナリオテスト | ✅ 実UI/UX |
| 異常系テスト | ✅ |
| 性能テスト | ✅ |

---

## 10. 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|------|-----------|----------|------|
| 2025/01/23 | 1.0 | 初版作成 | Claude |
| 2025/01/23 | 1.1 | expiryDate・有効期限テスト追加、AP・WiFi設定テスト追加（レビュー指摘対応） | Claude |
| 2025/01/23 | 1.2 | レビュー指摘対応: PIN Enable/Disable、デフォルト解決チェーン、NVS PINglobal初期化、Reconnect NTP同期、型不一致検証テスト追加 | Claude |
