# is01a - 汎用電池式温湿度BLEセンサー 設計書

**作成日**: 2025/12/22
**ベース**: archive_ISMS/ESP32/is01 (ISMS専用版)
**目的**: ISMS以外のプロジェクトでも使用可能な汎用版

---

## 1. デバイス概要

### 1.1 機能

- **温湿度センサー**: HT-30 (I2C)
- **BLEアドバタイズ**: 送信のみ（受信しない）
- **電池駆動**: DeepSleep運用（4分間隔）
- **初回アクティベーション**: WiFi接続してCIC取得

### 1.2 ユースケース

| 用途 | 説明 |
|------|------|
| 温度監視 | 倉庫・冷蔵庫・サーバールーム |
| 湿度監視 | 文書保管庫・美術品保管 |
| 環境モニタリング | 農業施設・温室 |

### 1.3 is01（ISMS版）との違い

| 項目 | is01 (ISMS版) | is01a (汎用版) |
|------|--------------|----------------|
| LacisID/CIC | **必須** | **必須** |
| AraneaRegister | **必須** | **必須** |
| 受信先 | is02/is03固定 | BLE受信側で設定 |
| TID | 固定（市山水産） | 設定可能 |
| OTA | 不可（電池式） | 不可（電池式） |

**重要**: LacisIDとCICはIoT動作の必須要件。初回起動時にAraneaRegisterで取得する。

---

## 2. ハードウェア仕様

### 2.1 ESP32-DevKitC GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 21 | I2C_SDA | HT-30/OLED SDA |
| 22 | I2C_SCL | HT-30/OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（長押し） |
| 26 | BTN_RESET | ファクトリーリセット（長押し） |

### 2.2 I2Cデバイス

| デバイス | アドレス | 用途 |
|---------|---------|------|
| HT-30 | 0x44 | 温湿度センサー |
| SSD1306 | 0x3C | OLED表示（128x64） |

### 2.3 電源仕様

- **電源**: 単3電池 x 2（3V）または18650リチウム電池
- **消費電力**:
  - アクティブ: ~80mA
  - DeepSleep: ~10μA
- **動作サイクル**: 4分間隔でWake→計測→BLE送信→Sleep

---

## 3. ソフトウェア設計

### 3.1 動作モード

```
┌─────────────────────────────────────────────────────────────┐
│              FIRST_BOOT (初回起動・アクティベーション)        │
│  1. WiFi接続                                                 │
│  2. LacisID生成                                              │
│  3. AraneaGateway登録 → CIC取得                              │
│  4. NVS保存（activated=1）                                   │
│  5. 通常動作へ移行                                           │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│              NORMAL_OPERATION (通常動作)                     │
│  1. DeepSleep復帰                                            │
│  2. I2C初期化（直列処理必須）                                │
│  3. HT-30計測                                                │
│  4. BLEアドバタイズ（3秒間）                                 │
│  5. DeepSleep（4分）                                         │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 初回アクティベーション

```cpp
// 初回起動時のみ実行
if (!settings.getInt("activated", 0)) {
    // WiFi接続
    wifi.connectWithSettings(&settings);

    // LacisID生成
    myLacisId = lacisGen.generate(PRODUCT_TYPE, PRODUCT_CODE);
    myMac = lacisGen.getStaMac12Hex();

    // AraneaGateway登録
    AraneaRegisterResult regResult = araneaReg.registerDevice(
        tid, DEVICE_TYPE, myLacisId, myMac, PRODUCT_TYPE, PRODUCT_CODE
    );

    if (regResult.ok) {
        myCic = regResult.cic_code;
        settings.setString("cic", myCic);
        settings.setInt("activated", 1);  // アクティベーション完了
    }
}
```

**重要**: CIC取得失敗でもBLEアドバタイズは必ず出す（is03で検知可能にする）。

### 3.3 I2C初期化（直列処理必須）

```cpp
// I2C初期化はコケやすいため直列実行必須
void initI2CDevices() {
    Wire.begin(SDA_PIN, SCL_PIN);
    delay(100);  // I2C安定化待ち

    // HT-30初期化
    if (!ht30.begin()) {
        Serial.println("HT-30 init failed");
    }
    delay(50);

    // OLED初期化
    if (!display.begin()) {
        Serial.println("OLED init failed");
    }
    delay(50);
}
```

**警告**: DeepSleep復帰後のI2C初期化は順序がシビア。並列処理禁止。

### 3.4 HT-30計測

```cpp
void readSensor() {
    if (ht30.readTemperatureHumidity()) {
        temperature = ht30.getTemperature();
        humidity = ht30.getHumidity();
    } else {
        temperature = NAN;
        humidity = NAN;
    }
}
```

---

## 4. BLE通信仕様

### 4.1 アドバタイズパケット（ISMSv1フォーマット）

```
┌────────────────────────────────────────────────────────────────┐
│ Prefix(1) │ LacisID(20) │ CIC(6) │ Temp(5) │ Hum(5) │ Batt(3) │
└────────────────────────────────────────────────────────────────┘
Total: 40 bytes
```

| フィールド | バイト | 説明 |
|-----------|--------|------|
| Prefix | 1 | 固定 "3" |
| LacisID | 20 | デバイス識別子 |
| CIC | 6 | 認証コード（0埋め6桁） |
| Temperature | 5 | 温度（例: "+25.5"） |
| Humidity | 5 | 湿度（例: "065.0"） |
| Battery | 3 | 電池残量%（例: "085"） |

### 4.2 アドバタイズ設定

```cpp
// BLEアドバタイズ設定
BLEAdvertisementData advData;
advData.setFlags(0x06);  // LE General Discoverable
advData.setName(advertiseName);  // ISMSv1フォーマットのペイロード

// 3秒間アドバタイズ
pAdvertising->start();
delay(3000);
pAdvertising->stop();
```

---

## 5. NVS設定項目

### 5.1 必須設定（AraneaRegister用）

| キー | 型 | 説明 |
|------|-----|------|
| `gate_url` | string | AraneaGateway URL |
| `tid` | string | テナントID |
| `tenant_lacisid` | string | テナントプライマリのlacisID |
| `tenant_email` | string | テナントプライマリのEmail |
| `tenant_pass` | string | テナントプライマリのパスワード |
| `tenant_cic` | string | テナントプライマリのCIC |
| `cic` | string | 自デバイスのCIC（取得後保存） |
| `activated` | int | アクティベーション完了フラグ（0/1） |

### 5.2 WiFi設定（初回のみ使用）

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `wifi_ssid` | string | - | WiFi SSID |
| `wifi_pass` | string | - | WiFi パスワード |

### 5.3 is01a固有設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `sleep_minutes` | int | 4 | DeepSleep間隔（分） |
| `ble_adv_sec` | int | 3 | BLEアドバタイズ時間（秒） |
| `device_name` | string | "is01a" | デバイス名 |

---

## 6. 起動シーケンス

### 6.1 初回起動

```
1. ストラッピングピン安定化
2. NVS初期化
3. activated=0を確認 → アクティベーションモードへ
4. WiFi接続
5. NTP同期（時刻取得）
6. LacisID生成（LacisIDGenerator）【必須】
7. AraneaGateway登録（AraneaRegister → CIC取得）【必須】
8. NVS保存（cic, activated=1）
9. 通常動作へ移行
```

### 6.2 通常起動（DeepSleep復帰）

```
1. DeepSleep復帰
2. NVS読み込み（lacisId, cic）
3. I2C初期化（直列処理）
4. HT-30計測
5. BLEアドバタイズ（3秒）
6. DeepSleep（4分）
```

**重要**: CIC未取得でもBLE広告は必ず出す。ペイロードのCIC部分は"000000"とする。

---

## 7. 開発ステップ

### Phase 1: 基本動作
- [ ] I2C初期化（直列処理）
- [ ] HT-30計測動作確認
- [ ] OLED表示

### Phase 2: BLE通信
- [ ] BLEアドバタイズ実装
- [ ] ISMSv1フォーマットペイロード構築
- [ ] 3秒間アドバタイズ確認

### Phase 3: 認証統合
- [ ] LacisID生成【必須】
- [ ] AraneaRegister連携【必須】
- [ ] CIC取得・NVS保存

### Phase 4: 省電力
- [ ] DeepSleep実装
- [ ] 4分間隔動作確認
- [ ] 電池寿命測定

---

## 8. global/モジュール使用計画

| モジュール | 使用 | 備考 |
|-----------|------|------|
| WiFiManager | △ | 初回アクティベーション時のみ |
| SettingManager | ○ | NVS永続化 |
| DisplayManager | ○ | OLED表示 |
| NtpManager | △ | 初回アクティベーション時のみ |
| LacisIDGenerator | **○必須** | lacisID生成（IoT動作に必須） |
| AraneaRegister | **○必須** | CIC取得（IoT動作に必須） |
| Operator | ○ | 状態機械 |

**OTA不可**: 電池駆動のためOTAは対応しない。ファームウェア更新はUSB経由。

---

## 9. 注意事項

### 9.1 I2C処理

- **I2C処理は必ず直列実行**（並列禁止）
- DeepSleep復帰→I2C初期化の順序がシビア
- 各デバイス初期化間にdelay()を入れる

### 9.2 CIC未取得時の動作

- CIC未取得でもBLEアドバタイズは**必ず出す**
- ペイロードのCIC部分は"000000"とする
- is03/is02で検知可能にするため

### 9.3 電池残量

- ADCで電池電圧を計測
- 3.0V以下で警告、2.7V以下で停止

---

## 10. 参照

- **is01 (ISMS版)**: `archive_ISMS/ESP32/is01/is01.ino`
- **is02 (BLE受信側)**: `code/ESP32/is02a/`
- **global モジュール**: `code/ESP32/global/src/`
- **役割分担ガイド**: `code/ESP32/______MUST_READ_ROLE_DIVISION______.md`
