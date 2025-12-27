# important_designersMemo.md

**修正履歴**
- Before: WDT/バックオフ/断片化対策や最新is04a/is05a固有API分離、force送信運用が未反映。
- After: 3sタイムアウト＋30sバックオフ・typeSpecific API分離・force送信（is04a）など現行実装を反映。is01/is02は参考情報として残し、未実装/未検証である旨を明記。

**設計者メモ - ESP32デバイス実装の重要指針**

---

## 1. 設定管理の基本方針

### 1-1. ハードコード禁止

**以下の項目はハードコードせず、settingManagerで管理すること：**

- テナント情報（TID, lacisId, CIC, email, pass）
- エンドポイントURL（relay.primary, relay.secondary, cloud endpoint）
- Wi-Fi設定（SSID, password）
- デバイス固有設定（hostname, rebootSchedule等）

### 1-1-1. 実装差分とTODO
- 共通: WiFi未接続時は送信を即return、HTTPタイムアウトは3s（現行実装済み）。3連続失敗で30sバックオフ（is04a/is05a/is10実装済み）。
- String連結による断片化防止: 送信系ペイロードは StaticJsonDocument + serializeJson + String::reserve() を徹底（Web UI生成の断片化は未対策）。
- Web UI生成は PROGMEM 固定文字列を優先し、頻繁な String 連結を避ける（未着手のため要検討）。

### 1-2. settingManager の必須キー

```
wifi           : SSID/Password候補リスト
hostname       : mDNS/表示用ホスト名
relay_pri      : Zero3 primary URL (192.168.96.201:8080)
relay_sec      : Zero3 secondary URL (192.168.96.202:8080)
endpoint_cloud : クラウドエンドポイント（将来用）
tid            : テナントID
tenant_lacisid : テナントPrimary lacisId
tenant_email   : テナント認証メール
tenant_cic     : テナントCIC
tenant_pass    : テナントパスワード
gate_url       : araneaDeviceGate URL
```

### 1-3. 設定の変更方法

- **HTTPManager Web UI経由**で変更可能にする
- 初回起動時にデフォルト値を投入
- NVS/SPIFFSに永続化

---

## 2. デバイス別仕様

### 2-1. ar-is01（電池式BLEセンサー）

**役割：** 温湿度をBLE Advertisingで送信。Wi-Fiは初回アクティベートのみ。

**動作モード：**
- PROVISION（未登録）: WiFi接続 → araneaRegister → CIC取得
- RUN（通常）: 3分周期で起床→測定→表示→広告→Sleep
- FAILSAFE: CIC無しでも広告は出す（Zero3で受信確認優先）

**重要：**
- I2C処理は必ず直列実行（並列禁止）
- DeepSleep復帰→I2C初期化の順序がシビア
- OTA不可で固める

### 2-2. ar-is02（BLEゲートウェイ + 自己センサー）※参考・未更新

**役割：**
- is01のBLEアドバタイズを受信
- Wi-Fi経由でZero3/クラウドへ中継
- **is02自身も温湿度センサー（HT-30）を持つ**

**送信形式：**
- BLEで受信したis01データ → is01のlacisIdで送信、meta.gatewayIdにis02のlacisId
- is02自身のセンサーデータ → is02のlacisIdで送信、meta.direct=true

**設定（settingManager経由）：**
```cpp
relay_pri      : "http://192.168.96.201:8080/api/events"
relay_sec      : "http://192.168.96.202:8080/api/events"
tid            : "T2025120608261484221"
tenant_lacisid : "12767487939173857894"
tenant_email   : "info+ichiyama@neki.tech"
tenant_cic     : "263238"
tenant_pass    : "dJBU^TpG%j$5"
gate_url       : "https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate"
```

**クラウドエンドポイント：**
| 機能 | URL |
|------|-----|
| デバイス登録 | https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate |
| 状態レポート | https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport |

### 2-3. ar-is04（接点出力 / HTTP制御）

**役割：** GPIO12/14の接点をパルス制御。入力1/2はPULLDOWN + inverted(true)で立ち上がり検出。

**制御経路（現行実装）：**
- 共通API `/api/status` `/api/config` に typeSpecific ブロックを付与。
- 固有API: `/api/is04a/pulse`（パルス実行）, `/api/is04a/trigger`（トリガーアサイン）。
- 送信: StateReporterがrelay(Primary/Secondary)→Cloudの順でHTTP送信。心拍/入力変化は最小1s、パルス開始/終了は`force=true`で即時送信。3sタイムアウト＋30sバックオフ。

### 2-4. ar-is05（8chリード入力）

**役割：** 窓/ドアの開閉状態を検知してRelay/Cloudへ送信（8ch）。

**制御/設定経路（現行実装）：**
- `/api/channels`(GET), `/api/channel`(GET/POST: chとmode/name/polarity), `/api/pulse`(POST)、`/api/webhook/config|test`(POST)。
- 送信は変化時と心拍（最小1s間隔）。StateReporter/Webhookとも3sタイムアウト＋30sバックオフ。Webhook testはplatform指定に未対応の既知課題あり。

---

## 3. 共通モジュール一覧

| Module | is01 | is02/04/05 | Purpose |
|--------|------|------------|---------|
| lacisIDGenerator | ○ | ○ | STA MACからlacisID生成 |
| araneaRegister | △(初回のみ) | ○ | cic_code取得・保存 |
| wifiManager | △(初回のみ) | ○ | cluster1-3への接続試行 |
| displayManager | ○ | ○ | I2C OLED 128x64表示 |
| ntpManager | × | ○ | 時刻同期 |
| settingManager | ○(NVS) | ○(SPIFFS) | 設定永続化 |
| HTTPManager | × | ○ | Web UI・API |
| otaManager | × | ○ | OTA更新 |
| SPIFFSManager | × | ○ | ファイルシステム |
| Operator | ○ | ○ | 状態機械・競合制御 |

### 3-1. 追加注意（現行実装と整合）
- IOController/ChannelManager（is05a）: mode切替時の禁止ピン/パルス中チェックを遵守。デバウンスは`millis()`経過で判定。StateReporter/Webhookとも3sタイムアウト＋30sバックオフ。
- TriggerManager（is04a）: force送信（sendStateReport(true)）で心拍間隔をスキップ。StateReporterは3sタイムアウト＋30sバックオフ。
- HTTP/Webhook/StateReporter: 全デバイスでWiFi未接続時は即return。断片化防止のため StaticJsonDocument + reserve を使用。

---

## 4. LacisID 形式

```
[Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁] = 20文字
例: 3 + 001 + AABBCCDDEEFF + 0096 = "3001AABBCCDDEEFF0096"
```

**BLE受信側での再構成：**
```
lacisId = "3" + zeroPad(productType, 3) + macToHex(mac) + zeroPad(productCode, 4)
```

---

## 5. 送信先フォールバック

```
1. relay.primary (is03-1: 192.168.96.201) へ送信
2. 失敗 → relay.secondary (is03-2: 192.168.96.202) へ送信
3. 連続3回失敗 → 30sバックオフ（ローカルバッファ保持は未実装、スキップのみ）
```

---

## 6. バッチ送信仕様（is02 ※参考・未更新）

- **間隔：** 30秒ごと
- **最大バッファ：** 50件
- **JSON形式：**

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
    "rssi": -65,
    "observedAt": "2025-01-15T10:30:00Z",
    "gatewayId": "3002FFEEDDCCBBAA0096"
  }
}
```

---

## 7. 開発時の注意点

1. **GPIO5**: I2C系デバイス動作にはGPIO5をHIGHにする必要あり
2. **NVSクリア**: デフォルト値変更時は `settings.clear()` を一時的に有効化
3. **NimBLE 2.x API**: `NimBLEScanCallbacks` を使用、`setScanCallbacks()` で登録
4. **重複排除**: is02側は不要、is03側で処理（is02は参考情報扱い）
