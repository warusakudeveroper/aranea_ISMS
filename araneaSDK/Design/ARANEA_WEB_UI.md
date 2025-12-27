# AraneaWebUI - デバイス共通Web UIフレームワーク

> **Version**: 1.6.0
> **Last Updated**: 2025-12-27
> **Status**: Production

---

## 概要

AraneaWebUIは全araneaDevicesで共通利用されるWeb UIフレームワークです。
ESP32上で動作し、設定管理・状態監視・デバイス制御のためのWebインターフェースを提供します。

### 設計原則

1. **共通コンポーネント + 機種固有拡張** - 継承による拡張ポイント
2. **MUI 7系準拠** - 統一されたクリーンなデザイン
3. **Single HTML** - CSS/JSを含む単一HTMLをPROGMEM埋め込み
4. **CIC認証** - 全APIにCICパラメータ認証

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│              AraneaWebUI (基底クラス)                    │
│              code/ESP32/global/src/                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 共通タブ (5タブ):                                  │  │
│  │   Status   - デバイス情報、Live Status、温度       │  │
│  │   Network  - WiFi 6スロット、AP設定、スキャン機能 │  │
│  │   Cloud    - 登録状態、MQTT接続                   │  │
│  │   Tenant   - TID/FID/deviceName                   │  │
│  │   System   - Reboot、Factory Reset、OTA           │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 共通API:                                           │  │
│  │   GET  /api/status?cic=XXXXXX                     │  │
│  │   GET  /api/config?cic=XXXXXX                     │  │
│  │   POST /api/wifi/scan                             │  │
│  │   GET  /api/wifi/scan/result                      │  │
│  │   POST /api/network/save                          │  │
│  │   POST /api/cloud/save                            │  │
│  │   POST /api/tenant/save                           │  │
│  │   POST /api/system/reboot                         │  │
│  │   GET  /api/speeddial                             │  │
│  │   POST /api/speeddial                             │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                         │
                         │ 継承 (public AraneaWebUI)
                         ▼
┌─────────────────────────────────────────────────────────┐
│           HttpManagerIs** (派生クラス)                   │
│           code/ESP32/is**/HttpManagerIs**.cpp           │
│  ┌───────────────────────────────────────────────────┐  │
│  │ オーバーライド可能メソッド:                        │  │
│  │   generateTypeSpecificTabs()          // タブ追加 │  │
│  │   generateTypeSpecificTabContents()   // HTML     │  │
│  │   generateTypeSpecificJS()            // JS       │  │
│  │   registerTypeSpecificEndpoints()     // API追加  │  │
│  │   getTypeSpecificStatus()             // Status拡張│  │
│  │   getTypeSpecificConfig()             // Config拡張│  │
│  │   generateTypeSpecificSpeedDial()     // SpeedDial│  │
│  │   applyTypeSpecificSpeedDial()        // 適用処理 │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 機種別タブ構成

| デバイス | deviceType | 共通タブ | 機種固有タブ | 用途 |
|----------|------------|----------|--------------|------|
| is02a | ar-is02a | 5タブ | Gateway | BLEゲートウェイ |
| is04a | ar-is04a | 5タブ | Control | GPIO接点出力 |
| is05a | ar-is05a | 5タブ | Channels | 8ch接点入力 |
| is06a | ar-is06a | 5タブ | Sensors | 温湿度センサー |
| **is10** | ar-is10 | 5タブ | Inspector, Routers | ルーター監視 |
| is10t | ar-is10t | 5タブ | Cameras | Tapoカメラ監視 |

---

## 依存モジュール

### 必須依存（AraneaWebUI.h）

```cpp
#include <Arduino.h>
#include <WebServer.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <SPIFFS.h>
#include <Preferences.h>
#include <esp_system.h>
#include "SettingManager.h"  // ★必須
```

### SettingManager 必須キー

AraneaWebUIが正常動作するために必要なSettingManagerキー：

```
[Network]
wifi[0-5].ssid    # WiFi SSID (6スロット)
wifi[0-5].pass    # WiFi Password
hostname          # デバイスホスト名
ntp_server        # NTPサーバー (default: pool.ntp.org)
ap_ssid           # APモードSSID
ap_pass           # APモードPassword
ap_timeout        # APモードタイムアウト(秒)

[Cloud]
registered        # 登録済みフラグ (bool)
lacis_id          # LacisID (20桁)
cic_code          # CIC (6桁)

[Tenant]
tid               # TenantID (T + 19桁)
fid               # FacilityID (4桁)
device_name       # デバイス名 (任意文字列)
```

---

## UIデザイン規約

### カラーパレット（MUI 7系 - 変更禁止）

```css
:root {
  /* 背景 */
  --bg-primary: #f8fafc;
  --bg-card: #ffffff;
  --bg-secondary: #f1f5f9;

  /* テキスト */
  --text-primary: #1e293b;
  --text-muted: #64748b;

  /* ボーダー・アクセント */
  --border: #e2e8f0;
  --accent: #3b82f6;

  /* ステータス */
  --success: #22c55e;
  --warning: #f59e0b;
  --danger: #ef4444;
}
```

### CSSクラス

| クラス | 用途 |
|--------|------|
| `.tab` / `.tab.active` | タブボタン |
| `.tab-content` | タブコンテンツ |
| `.card` / `.card-title` | カード |
| `.btn` / `.btn-primary` / `.btn-danger` | ボタン |
| `.form-group` / `.form-row` | フォーム |
| `.status-grid` / `.status-item` | ステータス表示 |
| `.modal` / `.modal-content` | モーダル |
| `.wifi-pair` / `.scan-pick` | WiFiスロット |

---

## 実装ガイド

### 新規デバイス追加手順

#### 1. HttpManagerIs**.h 作成

```cpp
#ifndef HTTP_MANAGER_IS**_H
#define HTTP_MANAGER_IS**_H

#include "AraneaWebUI.h"

class HttpManagerIs** : public AraneaWebUI {
public:
  void begin(SettingManager* settings, int port = 80) override;

protected:
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;
  void registerTypeSpecificEndpoints() override;
  void getTypeSpecificStatus(JsonObject& obj) override;
};

#endif
```

#### 2. 機種固有タブ実装

```cpp
String HttpManagerIs**::generateTypeSpecificTabs() {
  return R"(<div class="tab" data-tab="custom" onclick="showTab('custom')">Custom</div>)";
}

String HttpManagerIs**::generateTypeSpecificTabContents() {
  return R"HTML(
<div id="tab-custom" class="tab-content">
  <div class="card">
    <div class="card-title">Custom Settings</div>
    <div class="form-group">
      <label>Setting 1</label>
      <input type="text" id="c-setting1">
    </div>
    <button class="btn btn-primary" onclick="saveCustom()">Save</button>
  </div>
</div>
)HTML";
}

String HttpManagerIs**::generateTypeSpecificJS() {
  return R"JS(
async function saveCustom() {
  const data = { setting1: document.getElementById('c-setting1').value };
  await post('/api/custom/save', data);
  toast('Saved');
}
)JS";
}
```

#### 3. 機種固有API登録

```cpp
void HttpManagerIs**::registerTypeSpecificEndpoints() {
  server_->on("/api/custom/save", HTTP_POST, [this]() {
    if (!checkAuth()) { requestAuth(); return; }
    // 処理
    server_->send(200, "application/json", "{\"ok\":true}");
  });
}
```

#### 4. メインスケッチ初期化

```cpp
#include "HttpManagerIs**.h"

HttpManagerIs** webUI;
SettingManager settings;

void setup() {
  settings.begin();
  webUI.begin(&settings, 80);

  AraneaDeviceInfo info;
  info.deviceType = "ar-is**";
  info.modelName = "Custom Device";
  info.lacisId = settings.getLacisId();
  info.cic = settings.getCic();
  // ... 他の情報設定 ...
  webUI.setDeviceInfo(info);
}

void loop() {
  webUI.handle();
}
```

---

## WiFiスキャン機能 (v1.6.0)

### API

```
POST /api/wifi/scan          → {"ok":true,"status":"started"}
GET  /api/wifi/scan/result   → {"ok":true,"status":"complete","networks":[...]}
```

### UIフロー

1. **Scanボタン** - ヘッダー右に配置、全体スキャン開始
2. **▼ボタン** - 各スロット横、特定スロット用スキャン
3. **モーダル** - 検出ネットワーク一覧表示（SSID/RSSI/Ch/セキュリティ）
4. **選択** - SSIDがスロットに自動入力、Password欄にフォーカス

---

## チップ温度監視 (v1.6.0)

Status APIに`system.chipTemp`を追加。ESP32内部センサー（±5°C精度）で異常発熱を検知。

```json
{
  "system": {
    "chipTemp": 74.4
  }
}
```

---

## SpeedDial設定

一括設定インポート/エクスポート機能。INI形式テキストで設定を転送。

```ini
[Network]
wifi1.ssid=cluster1
wifi1.pass=isms12345@
hostname=ar-is10-ABCDEF

[Cloud]
registered=true

[Tenant]
tid=T2025120608261484221
fid=9000
device_name=Router Monitor

[Inspector]
scan_interval=60
ssh_timeout=90000
```

---

## ファイル配置

```
code/ESP32/
├── global/
│   ├── src/
│   │   ├── AraneaWebUI.cpp      # 共通UI (68KB)
│   │   ├── AraneaWebUI.h        # v1.6.0
│   │   ├── SettingManager.cpp   # 設定管理
│   │   └── SettingManager.h
│   └── library.properties
├── is02a/HttpManagerIs02a.*
├── is04a/HttpManagerIs04a.*
├── is05a/HttpManagerIs05a.*
├── is06a/HttpManagerIs06a.*
├── is10/HttpManagerIs10.*       # ← リファレンス実装
└── is10t/HttpManagerIs10t.*
```

---

## バージョン履歴

| Version | Date | Changes |
|---------|------|---------|
| **1.6.0** | 2025-12-27 | WiFiスキャンUI、チップ温度表示追加 |
| 1.5.0 | 2025-12-24 | SpeedDial/WiFi API共通化 |
| 1.4.0 | 2025-12-20 | CIC認証実装 |
| 1.3.0 | 2025-12-18 | 基本タブ構成確立 |

---

## 禁止事項

1. **カラーテーマの改変禁止** - MUI 7系の統一デザインを維持
2. **共通タブの独自改変禁止** - AraneaWebUI.cppでのみ管理
3. **CIC認証バイパス禁止** - 全APIで`checkAuth()`必須
4. **PROGMEM外部化禁止** - 単一バイナリ原則維持
