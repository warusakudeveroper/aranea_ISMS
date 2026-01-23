# IS06S 基本仕様書

**製品名**: Aranea Relay & Switch Controller
**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**最終更新**: 2025/01/23
**バージョン**: 1.3

---

## 1. 製品概要

### 1.1 定義

IS06SはaraneaDeviceシリーズのリレー・スイッチコントローラです。
ESP32DevkitC(Wroom32)をコア基盤として使用し、以下の機能を提供します。

### 1.2 主要機能

| 機能カテゴリ | 機能 | 説明 |
|-------------|------|------|
| **出力制御** | Digital Output | ON/OFF出力（リレー駆動） |
| **出力制御** | PWM Output | 調光出力（0-100%） |
| **入力制御** | Digital Input | 物理スイッチ入力 |
| **通信** | WiFi | 最大6 SSID対応、APモード |
| **通信** | MQTT | IoT制御・状態同期 |
| **通信** | HTTP | Web UI・REST API |
| **表示** | I2C OLED | 状態表示（0.96inch 128x64） |
| **更新** | OTA | HTTP経由ファームウェア更新 |

### 1.3 主用途

| 用途 | 対応PIN | 動作モード |
|------|---------|----------|
| 自動販売機制御 | CH1-CH4 | Digital (Momentary) |
| 電磁錠（施錠/解錠） | CH1-CH4 | Digital (Momentary) |
| 照明ON/OFF | CH1-CH4 | Digital (Alternate) |
| 調光照明 | CH1-CH4 | PWM (Slow/Rapid) |
| 物理スイッチ連動 | CH5-CH6 | Digital Input |

---

## 2. ハードウェアコンポーネント

### 2.1 userObjectDetailフォーマット

```json
{
  "hardware components": {
    "SBC": "ESP32Wroom32(ESP32E,DevkitC)",
    "I2COLED": "0.96inch_128*64_3.3V",
    "baseboard": "aranea_04"
  }
}
```

**注意**: キー名は`"hardware components"`（スペース入り）。SSoT: DesignerInstructions.md準拠。

### 2.2 電気的仕様

| 項目 | 仕様 |
|------|------|
| 動作電圧 | 3.3V (ESP32) |
| 出力電圧 | 3.3V CMOS |
| PWM解像度 | 8bit (0-255) |
| PWM周波数 | 5kHz |
| 入力方式 | INPUT_PULLDOWN |

---

## 3. PIN仕様

### 3.1 PIN一覧（MECE分類）

| 機能名称 | Type | GPIO | 電気仕様 | 説明 |
|---------|------|------|----------|------|
| CH1 | D/P | 18 | 3.3V OUTPUT/PWM | Digital or PWM出力 |
| CH2 | D/P | 05 | 3.3V OUTPUT/PWM | Digital or PWM出力 |
| CH3 | D/P | 15 | 3.3V OUTPUT/PWM | Digital or PWM出力 |
| CH4 | D/P | 27 | 3.3V OUTPUT/PWM | Digital or PWM出力 |
| CH5 | I/O | 14 | 3.3V INPUT/OUTPUT | Digital入力 or 出力 |
| CH6 | I/O | 12 | 3.3V INPUT/OUTPUT | Digital入力 or 出力 |
| Reconnect | System | 25 | 3.3V INPUT | WiFi再接続トリガー |
| Reset | System | 26 | 3.3V INPUT | ファクトリーリセット |
| I2C SDA | I2C | 21 | 3.3V | OLED データ |
| I2C SCL | I2C | 22 | 3.3V | OLED クロック |

### 3.2 PINタイプ定義

#### 3.2.1 D/P Type (Digital/PWM Output)

**適用PIN**: GPIO 18, 05, 15, 27 (CH1〜CH4)

このタイプのPINでは以下の2モードを切替可能：

| モード | 説明 | 設定値 |
|--------|------|--------|
| digitalOutput | ON/OFF出力 | type: "digitalOutput" |
| pwmOutput | 調光出力（0-100%） | type: "pwmOutput" |

#### 3.2.2 I/O Type (Digital Input/Output)

**適用PIN**: GPIO 14, 12 (CH5〜CH6)

このタイプのPINでは以下の2モードを切替可能：

| モード | 説明 | 設定値 |
|--------|------|--------|
| digitalOutput | ON/OFF出力 | type: "digitalOutput" |
| digitalInput | 物理スイッチ入力 | type: "digitalInput" |

**注意**: I/OタイプはPWM出力に対応しません。

#### 3.2.3 System Type

**適用PIN**: GPIO 25 (Reconnect), GPIO 26 (Reset)

| PIN | 動作 | 長押し時間 |
|-----|------|----------|
| GPIO25 | WiFi再接続・NTP同期 | 5,000ms |
| GPIO26 | NVS/SPIFFSリセット | 15,000ms |

---

## 4. データ構造（userObjectDetail）

### 4.1 構造概要（MECE）

```
userObjectDetail
├── firmware             // ファームウェア情報
├── config               // 設定（静的）
│   ├── PINSettings      //   PIN個別設定
│   ├── PINglobal        //   PIN共通デフォルト
│   ├── globalSettings   //   全体設定
│   └── hardware components // ハードウェア情報（スペース入りキー名）
├── status               // オンライン状態
└── network              // ネットワーク情報

※ PINState, globalState は araneaDeviceStates コレクションで管理（状態キャッシュ）
```

### 4.2 PINSettings（PIN設定）

**デフォルト値参照ルール**:
- 各フィールドが空欄または未設定の場合、`PINglobal`の対応する値を参照
- `PINglobal`も未設定の場合、`araneaSettings`の初期値を使用
- ハードコードは排除し、全デフォルト値はPINglobal経由で取得

#### 4.2.1 Digital Output設定

```json
{
  "PINSettings": {
    "CH1": {
      "enabled": "true",
      "type": "digitalOutput",
      "name": "照明スイッチ",
      "stateName": ["on:点灯", "off:消灯"],
      "actionMode": "Alt",
      "defaultValidity": "3000",
      "defaultDebounce": "3000",
      "defaultexpiry": "1"
    }
  }
}
```

| フィールド | 型 | 必須 | 未設定時参照先 | 説明 | 範囲 |
|-----------|-----|------|---------------|------|------|
| **enabled** | string | ○ | - | PIN機能の有効/無効 | "true"/"false" |
| type | string | ○ | - | "digitalOutput" | 固定値 |
| name | string | ○ | - | 表示名（任意） | 1-32文字 |
| stateName | string[] | ○ | - | on/off状態の表示名 | 2要素 |
| actionMode | string | △ | PINglobal.Digital.defaultActionMode | "Mom"/"Alt" | Mom/Alt |
| defaultValidity | string | △ | PINglobal.Digital.defaultValidity | Momの動作時間(ms) | "10"-"60000" |
| defaultDebounce | string | △ | PINglobal.Digital.defaultDebounce | デバウンス時間(ms) | "0"-"10000" |
| defaultexpiry | string | △ | PINglobal.common.defaultexpiry | 有効期限(day)、**小文字** | "0"-"365" |

**enabled機能**: `enabled="false"`の場合、以下の動作となる：
- Web UIのPIN Controlタブで該当PINが操作不可（グレーアウト表示）
- MQTT/HTTPコマンドを受けても拒否（エラー応答）
- 物理入力からの連動トリガーも無視
- 用途：リレー未接続のPIN誤操作防止、メンテナンス時の一時無効化

**SSoT準拠**: フィールド名は`defaultexpiry`（全小文字）。値は文字列型。
**ハードコード禁止**: △の項目は未設定時にPINglobalを参照。ファームウェア内でのデフォルト値直接記述は禁止。

#### 4.2.2 PWM Output設定

```json
{
  "PINSettings": {
    "CH2": {
      "enabled": "true",
      "type": "pwmOutput",
      "name": "調光LED",
      "stateName": ["0:0%", "30:30%", "60:60%", "100:100%"],
      "actionMode": "Slow",
      "RateOfChange": "4000",
      "defaultexpiry": "1"
    }
  }
}
```

| フィールド | 型 | 必須 | 未設定時参照先 | 説明 | 範囲 |
|-----------|-----|------|---------------|------|------|
| **enabled** | string | ○ | - | PIN機能の有効/無効 | "true"/"false" |
| type | string | ○ | - | "pwmOutput" | 固定値 |
| name | string | ○ | - | 表示名（任意） | 1-32文字 |
| stateName | string[] | ○ | - | プリセット表示名 | 2-10要素 |
| actionMode | string | △ | PINglobal.PWM.defaultActionMode | "Slow"/"Rapid" | Slow/Rapid |
| RateOfChange | string | △ | PINglobal.PWM.defaultRateOfChange | 0-100変化時間(ms)、**大文字R** | "0"-"10000" |
| defaultexpiry | string | △ | PINglobal.common.defaultexpiry | 有効期限(day)、**小文字** | "0"-"365" |

**ハードコード禁止**: △の項目は未設定時にPINglobalを参照。

#### 4.2.3 Digital Input設定

```json
{
  "PINSettings": {
    "CH6": {
      "enabled": "true",
      "type": "digitalInput",
      "allocation": ["CH1", "CH2"],
      "name": "物理スイッチ",
      "triggerType": "Digital",
      "actionMode": "Mom",
      "defaultValidity": "3000",
      "defaultDebounce": "3000",
      "defaultexpiry": "1"
    }
  }
}
```

| フィールド | 型 | 必須 | デフォルト | 説明 | 範囲 |
|-----------|-----|------|-----------|------|------|
| **enabled** | string | ○ | "true" | PIN機能の有効/無効 | "true"/"false" |
| type | string | ○ | - | "digitalInput" | 固定値 |
| allocation | string[] | ○ | - | 連動先PIN（複数指定可） | CH1-CH4 |
| name | string | ○ | - | 表示名（任意） | 1-32文字 |
| triggerType | string | ○ | - | "Digital" or "PWM" | PIN側から継承 |
| actionMode | string | ○ | - | "Mom"/"Alt"/"rotate" | PIN側から継承 |
| defaultValidity | string | △ | - | PIN側から継承 | PIN側から継承 |
| defaultDebounce | string | △ | - | PIN側から継承 | PIN側から継承 |
| defaultexpiry | string | △ | - | PIN側から継承、**小文字** | PIN側から継承 |

**バリデーションルール**:
- `allocation`は同一typeのPINのみ指定可能
- digitalOutputとpwmOutputの混在は禁止
- **enabled="false"の場合**: Input検知を無視（連動先への信号送出なし）

### 4.3 PINState（PIN状態）

#### 4.3.1 Digital Output状態

```json
{
  "PINState": {
    "CH1": {
      "state": "on",
      "Validity": "0",
      "Debounce": "0",
      "expiryDate": "202601231100"
    }
  }
}
```

| フィールド | 型 | 説明 | 範囲 |
|-----------|-----|------|------|
| state | string | "on" or "off" | on/off |
| Validity | string | オーバーライド動作時間(ms)、"0"=無期限、**大文字V** | "0"-"60000" |
| Debounce | string | オーバーライドデバウンス(ms)、"0"=デフォルト、**大文字D** | "0"-"10000" |
| expiryDate | string | 有効期限 (YYYYMMDDHHmm) | 日時形式 |

#### 4.3.2 PWM Output状態

```json
{
  "PINState": {
    "CH2": {
      "state": "40",
      "expiryDate": "202601231100"
    }
  }
}
```

| フィールド | 型 | 説明 | 範囲 |
|-----------|-----|------|------|
| state | string | PWM値(%) | "0"-"100" |
| expiryDate | string | 有効期限 (YYYYMMDDHHmm) | 日時形式 |

#### 4.3.3 Digital Input状態

```json
{
  "PINState": {
    "CH6": {
      "state": "on",
      "Validity": "0",
      "Debounce": "0",
      "expiryDate": "202601231100"
    }
  }
}
```

**動作優先順位**:
1. `Validity` > `defaultexpiry` で動作
2. ただし `expiryDate` を超えることは不可
3. `expiryDate`でオーバーライドされた場合は`expiryDate`が使用される

**SSoT準拠**: フィールド名は`Validity`, `Debounce`（大文字始まり）、`defaultexpiry`（全小文字）。

### 4.4 globalSettings（全体設定）

```json
{
  "globalSettings": {
    "wifiSetting": {
      "SSID1": "cluster1",
      "PASS1": "isms12345@",
      "SSID2": "",
      "PASS2": "",
      "SSID3": "",
      "PASS3": "",
      "SSID4": "",
      "PASS4": "",
      "SSID5": "",
      "PASS5": "",
      "SSID6": "",
      "PASS6": ""
    },
    "APmode": {
      "APSSID": "IS06S_SETUP",
      "APPASS": "",
      "APsubnet": "192.168.250.0/24",
      "APaddr": "192.168.250.1",
      "exclusiveConnection": "true"
    },
    "network": {
      "DHCP": "true",
      "Static": {
        "gateway": "192.168.1.1",
        "subnet": "255.255.255.0",
        "staticIP": "192.168.1.100"
      }
    },
    "WEBUI": {
      "localUID": ["admin", "admin123"],
      "lacisOath": "false"
    }
  }
}
```

| セクション | フィールド | 型 | 説明 |
|-----------|-----------|-----|------|
| wifiSetting | SSID1-6, PASS1-6 | string | 最大6 SSID（順次接続試行） |
| APmode | APSSID | string | APモード時のSSID |
| APmode | APPASS | string | APモード時のパスワード（空=オープン） |
| APmode | APsubnet | string | APモード時のサブネット |
| APmode | APaddr | string | APモード時のIPアドレス |
| APmode | exclusiveConnection | string | 単一接続制限（"true"/"false"） |
| network | DHCP | string | "true"=DHCP, "false"=Static |
| network | Static | object | 静的IP設定（DHCP="false"時有効）、**大文字S** |
| WEBUI | localUID | string[] | [ユーザー名, パスワード] |
| lacisOath | lacisOath | string | lacisOAuth認証許可（"true"/"false"） |

**wifiSettingバリデーション**:
- SSID/PASSペアは順番に接続試行（SSID1→SSID2→...→SSID6）
- 空欄SSIDはスキップして次のSSIDを試行
- 全SSID空欄または全接続失敗時はAPモードで起動
- 接続成功後、接続失敗した場合は自動再接続試行

**exclusiveConnection動作**:
- "true": APモード時に単一クライアントのみ接続許可
- "false": 複数クライアント接続許可（原則"true"推奨）

### 4.5 PINglobal（PIN共通デフォルト設定）

**設計方針**:
- PINglobalはPIN設定のデフォルト値を一元管理する（ハードコード排除）
- 個別PINSettingsで未設定の場合、PINglobalの値を参照
- 初期値はaraneaSettings経由でNVSに書き込み、userObjectDetailと同期
- **SSoT位置**: `userObjectDetail.config.PINglobal`（globalSettingsとは独立）

```json
{
  "PINglobal": {
    "Digital": {
      "defaultValidity": "3000",
      "defaultDebounce": "3000",
      "defaultActionMode": "Mom"
    },
    "PWM": {
      "defaultRateOfChange": "4000",
      "defaultActionMode": "Slow"
    },
    "common": {
      "defaultexpiry": "1",
      "defaultExpiryEnabled": "false"
    },
    "rebootSchedule": {
      "enabled": "false",
      "time": "03:00",
      "interval": "daily"
    }
  }
}
```

#### 4.5.1 Digital共通デフォルト

| フィールド | 型 | デフォルト | 説明 | 範囲 |
|-----------|-----|-----------|------|------|
| defaultValidity | string | "3000" | Momの動作時間(ms) | "10"-"60000" |
| defaultDebounce | string | "3000" | デバウンス時間(ms) | "0"-"10000" |
| defaultActionMode | string | "Mom" | デフォルト動作モード | "Mom"/"Alt" |

#### 4.5.2 PWM共通デフォルト

| フィールド | 型 | デフォルト | 説明 | 範囲 |
|-----------|-----|-----------|------|------|
| defaultRateOfChange | string | "4000" | Slow変化時間(ms) | "0"-"10000" |
| defaultActionMode | string | "Slow" | デフォルト変化モード | "Slow"/"Rapid" |

#### 4.5.3 共通デフォルト

| フィールド | 型 | デフォルト | 説明 | 範囲 |
|-----------|-----|-----------|------|------|
| defaultexpiry | string | "1" | 有効期限(day) | "0"-"365" |
| defaultExpiryEnabled | string | "false" | 有効期限機能の有効/無効 | "true"/"false" |

#### 4.5.4 デフォルト値参照フロー

```
PINSettings.CHx.defaultValidity が設定されている場合
  → PINSettings.CHx.defaultValidity を使用

PINSettings.CHx.defaultValidity が未設定（空欄 or 存在しない）場合
  → PINglobal.Digital.defaultValidity を使用

PINglobal.Digital.defaultValidity も未設定の場合
  → araneaSettings初期値（NVS）を使用
```

**優先順位**: `PINSettings（個別）` > `PINglobal（共通）` > `araneaSettings（初期値）`

#### 4.5.5 araneaSettings依存（初期値書き込み）

```cpp
// araneaSettings初期化時に書き込まれるデフォルト値
// ハードコードはここに集約し、他のコードでは参照のみ
namespace AraneaSettingsDefaults {
  // Digital
  const char* DIGITAL_VALIDITY = "3000";
  const char* DIGITAL_DEBOUNCE = "3000";
  const char* DIGITAL_ACTION_MODE = "Mom";

  // PWM
  const char* PWM_RATE_OF_CHANGE = "4000";
  const char* PWM_ACTION_MODE = "Slow";

  // Common
  const char* DEFAULT_EXPIRY = "1";
  const char* DEFAULT_EXPIRY_ENABLED = "false";
}
```

**初期化フロー**:
1. ESP32起動時、araneaSettingsがNVSから設定読み込み
2. NVSにPINglobal設定が存在しない場合、AraneaSettingsDefaultsの値を書き込み
3. PINglobal設定がuserObjectDetailに反映され、mobes2.0と同期可能に

**注意**: ファームウェア内でのハードコード値はAraneaSettingsDefaultsのみに限定。Is06PinManager等では必ずPINglobal経由で値を取得すること。

### 4.6 globalState（全体状態）

```json
{
  "globalState": {
    "ipAddress": "192.168.1.100",
    "uptime": 3600,
    "rssi": -65,
    "heapFree": 120000,
    "lastReboot": "2025-01-23T10:00:00Z",
    "rebootReason": "power_on"
  }
}
```

---

## 5. インターフェース

### 5.1 Web UI

| タブ | 機能 |
|------|------|
| Dashboard | 状態概要表示 |
| PINControl | PIN操作（ON/OFF/PWM） |
| PINSetting | PIN設定変更 |
| Network | WiFi/AP設定 |
| System | OTA更新・リセット |

### 5.2 HTTP API

| エンドポイント | メソッド | 説明 |
|---------------|---------|------|
| /api/status | GET | 全PIN状態取得 |
| /api/pin/{ch}/state | GET | 特定PIN状態取得 |
| /api/pin/{ch}/state | POST | PIN状態変更 |
| /api/pin/{ch}/setting | GET | 特定PIN設定取得 |
| /api/pin/{ch}/setting | POST | PIN設定変更 |
| /api/settings | GET | globalSettings取得 |
| /api/settings | POST | globalSettings変更 |

### 5.3 MQTT

| トピック | 方向 | 説明 |
|---------|------|------|
| device/{lacisId}/command | Subscribe | コマンド受信 |
| device/{lacisId}/state | Publish | 状態変化通知 |
| device/{lacisId}/ack | Publish | コマンドACK |

---

## 6. 共通コンポーネント依存

| コンポーネント | 必須 | 用途 |
|---------------|------|------|
| LacisIDGenerator | **必須** | lacisID生成 |
| AraneaRegister | **必須** | CIC取得・デバイス登録 |
| WiFiManager | ○ | WiFi接続管理 |
| SettingManager | ○ | NVS永続化 |
| MqttManager | ○ | MQTT接続 |
| DisplayManager | ○ | I2C OLED表示 |
| NtpManager | ○ | NTP同期 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | HTTP OTA更新 |

---

## 7. テスト環境設定

```json
{
  "SSID情報": {
    "SSID1": "sorapia_facility_wifi",
    "PASS1": "JYuDzjhi",
    "subnet": "192.168.77.0/24",
    "gateway": "192.168.77.1",
    "環境情報": "Omadaクラウドコントローラ管理下"
  },
  "設置施設情報": {
    "facilityName": "Sorapia Villa Mt.FUJI front",
    "fid": "0100",
    "rid": ["villa1", "villa2", "villa3", "villa4"]
  },
  "テナント情報": {
    "tenantName": "mijeos.inc",
    "tid": "T2025120621041161827",
    "tenantPrimaryUser": "soejim@mijeos.com",
    "tenantPrimaryLacisID": "18217487937895888001",
    "tenantPrimaryCIC": "204965"
  }
}
```

---

## 8. スキーマ・SDK連携

### 8.1 スキーマ階層構造

araneaDeviceシステムでは以下のスキーマを管理します（SSoT: araneaSDK）：

```
typeSettings/araneaDevice/ar-is06s
├── metadata          // 製品メタデータ
├── stateSchema       // 状態スキーマ (PINState)
├── configSchema      // 設定スキーマ (PINSettings, globalSettings)
└── commandSchema     // コマンドスキーマ (set, pulse, etc.)

userObjects/{lacisId}
├── lacis             // 認証情報 (type, tid, cic)
├── activity          // アクティビティ
└── device            // デバイス基本情報 (MAC, productType, productCode)

userObjects/{lacisId}/userObject_detail
├── firmware          // ファームウェア情報
├── config            // PINSettings, PINglobal, globalSettings
├── status            // オンライン状態
└── network           // ネットワーク情報
```

### 8.2 Type登録情報

| 項目 | 値 |
|------|-----|
| **Type名** | ar-is06s |
| **displayName** | Relay & Switch Controller |
| **productType** | 006 |
| **productCode** | 0200 |
| **capabilities** | output, input, pwm, pulse, trigger |
| **semanticTags** | 接点出力, PWM, リレー, 物理入力, 調光 |

#### 8.2.1 ProductCode姉弟関係

ProductType=006を共有するデバイスは「姉弟デバイス」として管理されます。

| typeName | ProductCode | 説明 | 特徴 |
|----------|-------------|------|------|
| ar-is06a | **0001** | 6ch出力コントローラー（基本型） | 6ch全て出力のみ |
| ar-is06s | **0200** | 4ch D/P + 2ch I/O（拡張型） | 入力機能追加 |

**lacisIDへの影響**:
同一MACアドレスでも、ProductCodeが異なれば別デバイスとして識別される。
- is06a: `30061234567890AB0001`
- is06s: `30061234567890AB0200`

### 8.3 スキーマファイル

スキーマ定義ファイル: `araneaSDK/schemas/types/aranea_ar-is06s.json`

**stateSchema（状態）**:
```json
{
  "PINState": {
    "CH1": { "state": "on", "Validity": "0", "expiryDate": "..." },
    "CH2": { "state": "50", "expiryDate": "..." },
    ...
  },
  "globalState": {
    "ipAddress": "192.168.77.100",
    "uptime": 3600,
    "rssi": -65,
    "heapFree": 120000
  }
}
```

**configSchema（設定）**: 本仕様書セクション4のPINSettings, PINglobal, globalSettingsを参照

**commandSchema（コマンド）**:
| コマンド | パラメータ | 説明 |
|---------|-----------|------|
| set | ch, state, validity?, expiryDate? | PIN状態設定 |
| pulse | ch, durationMs? | パルス出力 |
| allOff | - | 全チャンネルOFF |
| config | ch, setting | PIN設定変更 |

### 8.4 deviceStateReport形式

デバイスからサーバーへの状態レポート形式：

```json
{
  "auth": {
    "tid": "T2025120621041161827",
    "lacisId": "30061234567890AB0200",
    "cic": "123456"
  },
  "report": {
    "type": "ar-is06s",
    "lacisId": "30061234567890AB0200",
    "state": {
      "PINState": { "CH1": {...}, "CH2": {...}, ... },
      "globalState": { "ipAddress": "...", "rssi": -65, ... }
    },
    "observedAt": "2025-01-23T10:00:00Z",
    "meta": {
      "firmwareVersion": "1.0.0",
      "uptime": 3600,
      "heap": 120000
    }
  }
}
```

### 8.5 userObject構造

**userObject (Firestore: userObjects/{lacisId})**:
```json
{
  "lacis": {
    "type_domain": "araneaDevice",
    "type": "ar-is06s",
    "tid": "T2025120621041161827",
    "permission": 10,
    "cic_code": "123456",
    "cic_active": true
  },
  "activity": {
    "created_at": "2025-01-23T00:00:00Z",
    "lastUpdated_at": "2025-01-23T10:00:00Z"
  },
  "device": {
    "macAddress": "1234567890AB",
    "productType": "006",
    "productCode": "0200"
  },
  "fid": ["0150"]
}
```

### 8.6 userObject_detail構造

**userObject_detail (Firestore: userObjects/{lacisId}/userObject_detail)**:
```json
{
  "firmware": {
    "version": "1.0.0",
    "buildDate": "2025-01-23T00:00:00Z",
    "modules": ["WiFi", "MQTT", "OTA", "WebUI"]
  },
  "config": {
    "PINSettings": { ... },
    "PINglobal": { ... },
    "globalSettings": { ... },
    "hardware components": { ... }
  },
  "status": {
    "online": true,
    "lastSeen": "2025-01-23T10:00:00Z",
    "rssi": -65,
    "heap": 120000
  },
  "network": {
    "ip": "192.168.77.100",
    "ssid": "sorapia_facility_wifi",
    "gateway": "192.168.77.1",
    "subnet": "255.255.255.0"
  }
}
```

### 8.7 lacisID形式

| 位置 | 桁数 | 内容 | IS06S値 |
|------|------|------|--------|
| 1 | 1桁 | Prefix (araneaDevice) | 3 |
| 2-4 | 3桁 | ProductType | 006 |
| 5-16 | 12桁 | MACアドレス (大文字HEX) | (デバイス固有) |
| 17-20 | 4桁 | ProductCode | 0200 |

**例**: `30061234567890AB0200` (20文字)

### 8.8 araneaDeviceGate登録フロー

```
┌──────────────┐    1. CIC取得リクエスト     ┌──────────────────┐
│    IS06S     │ ──────────────────────────► │ araneaDeviceGate │
│   Device     │                             │    (mobes2.0)    │
└──────┬───────┘                             └────────┬─────────┘
       │                                              │
       │                                              │ 2. lacisID生成
       │                                              │    CIC発行
       │                                              │    userObject作成
       │                                              ▼
       │         3. CIC返却                  ┌──────────────────┐
       │ ◄────────────────────────────────── │    Firestore     │
       │                                     │  userObjects/    │
       │                                     │  typeSettings/   │
       │                                     └──────────────────┘
       │
       │ 4. NVSにCIC保存
       │ 5. deviceStateReport開始
       ▼
┌──────────────┐
│ 状態レポート  │
│ 定期送信      │
└──────────────┘
```

---

## 9. MECE確認

本仕様書は以下の観点でMECE（漏れなくダブりなく）です：

### 9.1 PIN分類

| 分類 | 対象PIN | 完全網羅確認 |
|------|---------|-------------|
| D/P (Digital/PWM Output) | CH1-CH4 (GPIO18,05,15,27) | ✅ |
| I/O (Digital Input/Output) | CH5-CH6 (GPIO14,12) | ✅ |
| System | Reconnect, Reset (GPIO25,26) | ✅ |
| I2C | SDA, SCL (GPIO21,22) | ✅ |

### 9.2 データ構造分類

| 分類 | 対象 | 完全網羅確認 |
|------|------|-------------|
| 静的設定 | PINSettings, globalSettings | ✅ |
| 動的状態 | PINState, globalState | ✅ |
| ハードウェア情報 | hardwareComponents | ✅ |

### 9.3 アンアンビギュアス確認

- 全フィールドに型・デフォルト値・範囲を明示
- 曖昧な表現を排除し、具体的な値・条件を記載

---

## 10. 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|------|-----------|----------|------|
| 2025/01/23 | 1.0 | 初版作成（MECE・アンアンビギュアス化） | Claude |
| 2025/01/23 | 1.1 | スキーマ・SDK連携セクション拡充（userObject, userObject_detail, deviceStateReport, lacisID, 登録フロー） | Claude |
| 2025/01/23 | 1.2 | PINSettings全タイプにenabledフィールド追加、ProductCode姉弟関係セクション追加 | Claude |
| 2025/01/23 | 1.3 | Type名をar-is06sに統一、ProductCodeを0200に変更、PINglobal位置をconfig直下に修正 | Claude |
