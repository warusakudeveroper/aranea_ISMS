# AraneaWebUI 実装ガイド

新規デバイス向け設定画面の構築方法マニュアル

---

## 1. 概要

AraneaWebUIは、全Araneaデバイス（is02, is04, is05, is10等）で共通利用可能なWeb UI基盤クラスです。
共通機能（Status/Network/Cloud/Tenant/System）を提供し、派生クラスでデバイス固有の機能を追加できます。

### アーキテクチャ

```
AraneaWebUI (基底クラス)
    │
    ├── 共通タブ: Status / Network / Cloud / Tenant / System
    ├── 共通API: /api/status, /api/config, /api/common/*, /api/system/*
    ├── Basic認証
    ├── CICパラメータ認証（巡回ポーリング用）
    │
    └── 派生クラス (例: HttpManagerIs10)
         ├── デバイス固有タブ: Inspector / Routers
         ├── デバイス固有API: /api/is10/*
         └── デバイス固有ステータス
```

---

## 2. ファイル構成

```
generic/ESP32/global/src/
├── AraneaWebUI.h          # 基底クラス定義
├── AraneaWebUI.cpp        # 基底クラス実装
└── SettingManager.h       # 設定永続化（依存）

generic/ESP32/{device}/
├── HttpManager{Device}.h   # 派生クラス定義
├── HttpManager{Device}.cpp # 派生クラス実装
└── {device}.ino           # メインスケッチ
```

---

## 3. 基底クラス AraneaWebUI

### 3.1 主要構造体

```cpp
// デバイス情報（Statusタブ表示用）
struct AraneaDeviceInfo {
  String deviceType;      // "ar-is10" - デバイスタイプID
  String modelName;       // "Router Inspector" - モデル名
  String contextDesc;     // "ルーターの状態監視..." - 機能説明
  String lacisId;         // LacisID
  String cic;             // CIC認証コード
  String mac;             // MACアドレス
  String hostname;        // ホスト名
  String firmwareVersion; // ファームウェアバージョン
  String buildDate;       // ビルド日
  String modules;         // "WiFi,NTP,SSH,MQTT" カンマ区切り
};

// ネットワーク状態
struct AraneaNetworkStatus {
  String ip;
  String ssid;
  int rssi;
  String gateway;
  String subnet;
  bool apMode;
  String apSsid;
};

// システム状態
struct AraneaSystemStatus {
  String ntpTime;         // ISO8601形式
  bool ntpSynced;
  unsigned long uptime;   // 秒
  size_t heap;
  size_t heapLargest;
  int cpuFreq;            // MHz
  size_t flashSize;
};

// クラウド接続状態
struct AraneaCloudStatus {
  bool registered;
  bool mqttConnected;
  String lastStateReport;
  int lastStateReportCode;
  int schemaVersion;
};
```

### 3.2 基底クラスメソッド

```cpp
class AraneaWebUI {
public:
  // 初期化
  virtual void begin(SettingManager* settings, int port = 80);

  // ループ処理（WebServer.handleClient()）
  virtual void handle();

  // デバイス情報設定
  void setDeviceInfo(const AraneaDeviceInfo& info);

  // WebServerインスタンス取得
  WebServer* getServer();

  // コールバック登録
  void onSettingsChanged(void (*callback)());   // 設定変更時
  void onRebootRequest(void (*callback)());     // 再起動要求時
  void onDeviceNameChanged(void (*callback)()); // deviceName変更時（SSOT）

protected:
  // オーバーライド可能なメソッド
  virtual void getTypeSpecificStatus(JsonObject& obj);     // 固有ステータス追加
  virtual void getTypeSpecificConfig(JsonObject& obj);     // 固有設定追加
  virtual String generateTypeSpecificTabs();               // 固有タブHTML
  virtual String generateTypeSpecificTabContents();        // 固有タブ内容HTML
  virtual String generateTypeSpecificJS();                 // 固有JavaScript
  virtual void registerTypeSpecificEndpoints();            // 固有APIエンドポイント登録
};
```

---

## 4. 派生クラス実装手順

### 4.1 ヘッダファイル作成

```cpp
// HttpManagerIsXX.h
#ifndef HTTP_MANAGER_ISXX_H
#define HTTP_MANAGER_ISXX_H

#include <Arduino.h>
#include "AraneaWebUI.h"

class HttpManagerIsXX : public AraneaWebUI {
public:
  // 初期化（固有パラメータがあれば追加）
  void begin(SettingManager* settings, int port = 80);

  // 固有のステータス更新メソッド
  void setSensorValue(float value);

protected:
  // AraneaWebUI オーバーライド
  void getTypeSpecificStatus(JsonObject& obj) override;
  void getTypeSpecificConfig(JsonObject& obj) override;
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;
  void registerTypeSpecificEndpoints() override;

private:
  // 固有データ
  float sensorValue_ = 0.0;

  // 固有ハンドラ
  void handleSaveSensor();
};

#endif
```

### 4.2 実装ファイル作成

```cpp
// HttpManagerIsXX.cpp
#include "HttpManagerIsXX.h"

// ========================================
// 初期化
// ========================================
void HttpManagerIsXX::begin(SettingManager* settings, int port) {
  // 基底クラス初期化（共通エンドポイント登録含む）
  AraneaWebUI::begin(settings, port);

  Serial.printf("[HTTP-ISXX] Server started on port %d\n", port);
}

// ========================================
// 固有エンドポイント登録
// ========================================
void HttpManagerIsXX::registerTypeSpecificEndpoints() {
  server_->on("/api/isxx/sensor", HTTP_POST, [this]() {
    handleSaveSensor();
  });
}

// ========================================
// 固有ステータス（/api/status 用）
// ========================================
void HttpManagerIsXX::getTypeSpecificStatus(JsonObject& obj) {
  obj["sensorValue"] = sensorValue_;
  obj["threshold"] = settings_->getInt("isxx_threshold", 50);
}

// ========================================
// 固有設定（/api/config 用）
// ========================================
void HttpManagerIsXX::getTypeSpecificConfig(JsonObject& obj) {
  JsonObject sensor = obj.createNestedObject("sensor");
  sensor["threshold"] = settings_->getInt("isxx_threshold", 50);
  sensor["interval"] = settings_->getInt("isxx_interval", 1000);
}

// ========================================
// 固有タブ定義
// ========================================
String HttpManagerIsXX::generateTypeSpecificTabs() {
  return R"TABS(
<div class="tab" data-tab="sensor" onclick="showTab('sensor')">Sensor</div>
)TABS";
}

// ========================================
// 固有タブコンテンツ
// ========================================
String HttpManagerIsXX::generateTypeSpecificTabContents() {
  return R"HTML(
<!-- Sensor Tab -->
<div id="tab-sensor" class="tab-content">
<div class="card">
<div class="card-title">Sensor Status</div>
<div class="status-grid">
<div class="status-item"><div class="label">Current Value</div><div class="value" id="xx-value">-</div></div>
</div>
</div>
<div class="card">
<div class="card-title">Sensor Settings</div>
<div class="form-row">
<div class="form-group"><label>Threshold</label><input type="number" id="xx-threshold"></div>
<div class="form-group"><label>Interval (ms)</label><input type="number" id="xx-interval"></div>
</div>
<button class="btn btn-primary" onclick="saveSensor()">Save</button>
</div>
</div>
)HTML";
}

// ========================================
// 固有JavaScript
// ========================================
String HttpManagerIsXX::generateTypeSpecificJS() {
  return R"JS(
<script>
// 設定読み込み時に呼ばれる
function renderTypeSpecific(){
  const ts = cfg.typeSpecific || {};
  const sensor = ts.sensor || {};
  document.getElementById('xx-threshold').value = sensor.threshold || 50;
  document.getElementById('xx-interval').value = sensor.interval || 1000;
}

// ステータス更新時に呼ばれる（5秒間隔）
function refreshTypeSpecificStatus(s){
  const ts = s.typeSpecific || {};
  document.getElementById('xx-value').textContent = ts.sensorValue || '-';
}

// 設定保存
async function saveSensor(){
  await post('/api/isxx/sensor', {
    threshold: parseInt(document.getElementById('xx-threshold').value),
    interval: parseInt(document.getElementById('xx-interval').value)
  });
  toast('Sensor settings saved');
}
</script>
)JS";
}

// ========================================
// 固有ハンドラ
// ========================================
void HttpManagerIsXX::handleSaveSensor() {
  if (!server_->hasArg("plain")) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No body\"}");
    return;
  }

  StaticJsonDocument<256> doc;
  DeserializationError err = deserializeJson(doc, server_->arg("plain"));
  if (err) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Invalid JSON\"}");
    return;
  }

  if (doc.containsKey("threshold")) {
    settings_->setInt("isxx_threshold", doc["threshold"]);
  }
  if (doc.containsKey("interval")) {
    settings_->setInt("isxx_interval", doc["interval"]);
  }

  if (settingsChangedCallback_) settingsChangedCallback_();
  server_->send(200, "application/json", "{\"ok\":true}");
}

// ========================================
// ステータス更新
// ========================================
void HttpManagerIsXX::setSensorValue(float value) {
  sensorValue_ = value;
}
```

### 4.3 メインスケッチでの使用

```cpp
// isxx.ino
#include "HttpManagerIsXX.h"
#include "SettingManager.h"

#define FIRMWARE_VERSION "1.0.0"
#define BUILD_DATE __DATE__
#define ARANEA_DEVICE_TYPE "ar-isxx"

SettingManager settings;
HttpManagerIsXX httpMgr;

void setup() {
  Serial.begin(115200);

  // WiFi接続...
  // NTP同期...
  // CIC取得...

  // 設定読み込み
  settings.begin();

  // デバイス情報設定
  AraneaDeviceInfo info;
  info.deviceType = ARANEA_DEVICE_TYPE;
  info.modelName = "Sensor Monitor";            // モデル名
  info.contextDesc = "センサー値の監視と記録";    // 機能説明
  info.lacisId = getLacisId();
  info.cic = getCic();
  info.mac = getMacAddress();
  info.hostname = getHostname();
  info.firmwareVersion = FIRMWARE_VERSION;
  info.buildDate = BUILD_DATE;
  info.modules = "WiFi,NTP,Sensor,MQTT";

  httpMgr.setDeviceInfo(info);

  // コールバック登録
  httpMgr.onSettingsChanged([]() {
    Serial.println("[SETTINGS] Changed - reloading config");
    // 設定リロード処理
  });

  httpMgr.onDeviceNameChanged([]() {
    Serial.println("[WebUI] deviceName changed");
    sendDeviceStateReport();  // 即時送信
  });

  // HTTPサーバー開始
  httpMgr.begin(&settings, 80);
}

void loop() {
  httpMgr.handle();

  // センサー値更新
  float value = readSensor();
  httpMgr.setSensorValue(value);

  delay(10);
}
```

---

## 5. 共通API仕様

### 5.1 エンドポイント一覧

| メソッド | パス | 認証 | 説明 |
|---------|------|------|------|
| GET | / | Basic Auth | HTML UI表示 |
| GET | /?cic=XXXXXX | CIC | JSONステータス（巡回用） |
| GET | /api/status | なし | 全ステータスJSON |
| GET | /api/config | Basic Auth | 全設定JSON |
| POST | /api/common/network | Basic Auth | WiFi/ホスト名設定 |
| POST | /api/common/ap | Basic Auth | APモード設定 |
| POST | /api/common/cloud | Basic Auth | クラウド連携設定 |
| POST | /api/common/tenant | Basic Auth | テナント認証設定 |
| POST | /api/system/settings | Basic Auth | システム設定（deviceName等） |
| POST | /api/system/reboot | Basic Auth | 再起動 |
| POST | /api/system/factory-reset | Basic Auth | ファクトリーリセット |

### 5.2 共通設定キー

| キー | 型 | 説明 |
|------|-----|------|
| wifi_ssid1〜6 | String | WiFi SSID（6優先度） |
| wifi_pass1〜6 | String | WiFi パスワード |
| hostname | String | ホスト名 |
| ntp_server | String | NTPサーバー |
| ap_ssid | String | APモードSSID |
| ap_pass | String | APモードパスワード |
| ap_timeout | int | APタイムアウト（秒） |
| gate_url | String | AraneaDeviceGate URL |
| state_report_url | String | deviceStateReport URL |
| relay_pri | String | Zero3プライマリURL |
| relay_sec | String | Zero3セカンダリURL |
| tid | String | テナントID |
| fid | String | ファシリティID |
| tenant_lacisid | String | テナントプライマリLacisID |
| tenant_email | String | テナントプライマリEmail |
| tenant_cic | String | テナントプライマリCIC |
| device_name | String | デバイス名（SSOT） |
| ui_password | String | Basic認証パスワード |
| reboot_enabled | bool | 定期再起動有効 |
| reboot_time | String | 定期再起動時刻（HH:MM） |

---

## 6. 認証

### 6.1 Basic認証

`ui_password` 設定時、UIアクセスにBasic認証が必要になります。

```
ユーザー名: admin
パスワード: ui_password設定値
```

未設定時は認証なしでアクセス可能。

### 6.2 CIC認証（巡回ポーリング用）

URLパラメータでCICを指定すると、Basic認証をバイパスしてJSONステータスを取得可能。

```
GET http://192.168.x.x/?cic=397815
→ JSON status response
```

CIC不一致時は`401 Invalid CIC`を返却。

---

## 7. JavaScript連携ルール

### 7.1 必須関数

派生クラスのJSで定義すべき関数:

```javascript
// 設定読み込み完了時（/api/config レスポンス後）
function renderTypeSpecific() {
  const ts = cfg.typeSpecific || {};
  // 固有設定をフォームに反映
}

// ステータス更新時（5秒間隔）
function refreshTypeSpecificStatus(s) {
  const ts = s.typeSpecific || {};
  // 固有ステータスをUIに反映
}
```

### 7.2 利用可能なグローバル

```javascript
cfg          // /api/config レスポンスオブジェクト
post(url, data)  // POSTリクエスト送信
toast(msg)       // トースト通知表示
showTab(name)    // タブ切り替え
load()           // 設定再読み込み
```

---

## 8. CSS クラス

### 8.1 レイアウト

| クラス | 説明 |
|--------|------|
| .card | カードコンテナ |
| .card-title | カードタイトル |
| .status-grid | ステータスグリッド |
| .status-item | ステータス項目 |
| .form-group | フォームグループ |
| .form-row | フォーム行（横並び） |
| .btn-group | ボタングループ |

### 8.2 ボタン

| クラス | 説明 |
|--------|------|
| .btn | ボタン基本 |
| .btn-primary | プライマリボタン（青） |
| .btn-danger | 危険ボタン（赤） |
| .btn-sm | 小サイズボタン |

### 8.3 ステータス色

| クラス | 説明 |
|--------|------|
| .value.good | 正常（緑） |
| .value.warn | 警告（黄） |
| .value.bad | 異常（赤） |

---

## 9. 実装例: is10 (Router Inspector)

### 固有タブ

- **Inspector**: SSH設定、CelestialGlobe SSOT設定
- **Routers**: ルーター一覧管理（追加/編集/削除）

### 固有API

| パス | 説明 |
|------|------|
| POST /api/is10/inspector | Inspector設定保存 |
| POST /api/is10/router | ルーター追加/更新 |
| POST /api/is10/router/delete | ルーター削除 |

### 固有設定キー

| キー | 型 | 説明 |
|------|-----|------|
| is10_endpoint | String | CelestialGlobe URL |
| is10_celestial_secret | String | X-Celestial-Secret |
| is10_scan_interval_sec | int | スキャン間隔（秒） |
| is10_report_clients | bool | クライアントリスト送信 |
| is10_ssh_timeout | int | SSHタイムアウト（ms） |
| is10_retry_count | int | リトライ回数 |
| is10_router_interval | int | ルーター間インターバル（ms） |

### ルーター設定（SPIFFS）

ルーター設定は `/routers.json` にJSON配列で保存:

```json
[
  {
    "rid": "306",
    "ipAddr": "192.168.125.186",
    "sshPort": 65305,
    "username": "admin",
    "password": "****",
    "publicKey": "ssh-rsa ...",
    "osType": 1,
    "enabled": true
  }
]
```

---

## 10. deviceStateReport連携

### 10.1 deviceName フィールド

WebUIのSystemタブで設定した`deviceName`は、`deviceStateReport`の`state.deviceName`として送信されます。

```json
{
  "lacisId": "...",
  "type": "aranea_ar-is10",
  "state": {
    "deviceName": "本社1F-ルーター監視",
    "IPAddress": "192.168.3.91",
    ...
  }
}
```

### 10.2 即時送信

deviceName変更時は`onDeviceNameChanged`コールバックが呼ばれるため、即時送信が可能:

```cpp
httpMgr.onDeviceNameChanged([]() {
  sendDeviceStateReport();
});
```

### 10.3 サニタイズ

deviceNameは送信前に正規化:
1. trim（前後空白除去）
2. 制御文字（0x00-0x1F, 0x7F）除去
3. 連続空白を1つに圧縮
4. 最大64文字で切り捨て

---

## 11. トラブルシューティング

### UI表示されない

1. `httpMgr.begin()` が呼ばれているか確認
2. `httpMgr.handle()` がloop()内で呼ばれているか確認
3. WiFi接続状態を確認

### 固有タブが表示されない

1. `generateTypeSpecificTabs()` が空でないか確認
2. `showTab('tabname')` のタブ名が一致しているか確認

### 設定が保存されない

1. `settings_->setString/setInt` の後に保存されているか確認
2. SettingManagerの初期化を確認

### Basic認証が効かない

1. `ui_password` 設定を確認
2. ブラウザのキャッシュをクリア

---

## 12. バージョン履歴

| バージョン | 日付 | 変更内容 |
|-----------|------|----------|
| 1.0.0 | 2025-12-20 | 初版リリース（is10実装完了） |

---

## 関連ドキュメント

- [AraneaWebUI設計書](./aranea_web_ui_design.md)
- [AraneaDeviceGate仕様](./araneaDeviceGate_spec.md)
- [デバイスプロビジョニングフロー](./device_provisioning_flow.md)
