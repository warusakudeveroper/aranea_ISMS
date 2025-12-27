# AraneaWebUI アーキテクチャガイド

## 概要

AraneaWebUIは全araneaDevicesで共通利用されるWeb UIフレームワークです。
MUI 7系のクリーンなデザインを採用し、共通コンポーネントと機種固有コンポーネントの組み合わせで構成されます。

**現在のバージョン**: v1.6.0

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────┐
│           AraneaWebUI (基底クラス)              │
│         code/ESP32/global/src/                   │
│  ┌─────────────────────────────────────────┐    │
│  │ 共通タブ:                                │    │
│  │  - Status (デバイス情報/Live Status)    │    │
│  │  - Network (WiFi 6スロット/AP設定)      │    │
│  │  - Cloud (登録状態/MQTT)                │    │
│  │  - Tenant (TID/FID/deviceName)          │    │
│  │  - System (Reboot/Factory Reset)        │    │
│  └─────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────┐    │
│  │ 共通機能:                                │    │
│  │  - WiFiスキャン (Scan + モーダル選択)   │    │
│  │  - SpeedDial設定インポート/エクスポート │    │
│  │  - CIC認証付きAPI                        │    │
│  │  - チップ温度監視                        │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
                      │
                      │ 継承
                      ▼
┌─────────────────────────────────────────────────┐
│      HttpManagerIs** (派生クラス)               │
│      code/ESP32/is**/HttpManagerIs**.cpp        │
│  ┌─────────────────────────────────────────┐    │
│  │ 機種固有タブ (オーバーライド):          │    │
│  │  - generateTypeSpecificTabs()           │    │
│  │  - generateTypeSpecificTabContents()    │    │
│  │  - generateTypeSpecificJS()             │    │
│  │  - registerTypeSpecificEndpoints()      │    │
│  │  - getTypeSpecificStatus()              │    │
│  │  - getTypeSpecificConfig()              │    │
│  │  - generateTypeSpecificSpeedDial()      │    │
│  │  - applyTypeSpecificSpeedDial()         │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

---

## 機種別タブ構成

| デバイス | 共通タブ | 機種固有タブ |
|----------|----------|--------------|
| is02a    | Status/Network/Cloud/Tenant/System | Gateway |
| is04a    | Status/Network/Cloud/Tenant/System | Control (GPIO出力) |
| is05a    | Status/Network/Cloud/Tenant/System | Channels (8ch入力) |
| is06a    | Status/Network/Cloud/Tenant/System | Sensors |
| is10     | Status/Network/Cloud/Tenant/System | Inspector/Routers |
| is10t    | Status/Network/Cloud/Tenant/System | Cameras |

---

## 依存関係

### 必須モジュール（共通）

```cpp
// AraneaWebUI.h が依存するモジュール
#include <Arduino.h>
#include <WebServer.h>
#include <ArduinoJson.h>
#include <WiFi.h>
#include <SPIFFS.h>
#include <Preferences.h>
#include <esp_system.h>
#include "SettingManager.h"  // 必須: 設定永続化
```

### SettingManager 必須キー

AraneaWebUIが正常に動作するために、SettingManagerに以下のキーが必要です：

```cpp
// Network設定
wifi[0-5].ssid, wifi[0-5].pass  // WiFi 6スロット
hostname                         // ホスト名
ntp_server                       // NTPサーバー
ap_ssid, ap_pass, ap_timeout    // APモード設定

// Cloud設定
registered                       // 登録済みフラグ
mqtt_connected                  // MQTT接続状態
lacis_id, cic_code              // デバイス識別子

// Tenant設定
tid, fid, device_name           // テナント情報

// System設定（読み取り専用）
firmware_version, build_date    // ファームウェア情報
```

---

## UIデザインルール

### カラーテーマ（MUI 7系準拠）

```css
:root {
  --bg-primary: #f8fafc;      /* 背景メイン */
  --bg-card: #ffffff;          /* カード背景 */
  --bg-secondary: #f1f5f9;    /* セカンダリ背景 */
  --text-primary: #1e293b;    /* テキストメイン */
  --text-muted: #64748b;      /* テキストミュート */
  --border: #e2e8f0;          /* ボーダー */
  --accent: #3b82f6;          /* アクセントカラー */
  --success: #22c55e;         /* 成功 */
  --warning: #f59e0b;         /* 警告 */
  --danger: #ef4444;          /* 危険/エラー */
}
```

**重要**: カラーテーマの独自改変は禁止です。全デバイスで統一されたUIを維持してください。

### コンポーネント規約

- **タブ**: `.tab` / `.tab.active`
- **カード**: `.card` / `.card-title`
- **ボタン**: `.btn` / `.btn-primary` / `.btn-danger`
- **フォーム**: `.form-group` / `.form-row`
- **ステータス**: `.status-grid` / `.status-item`
- **モーダル**: `.modal` / `.modal-content`

---

## 実装手順（新規デバイス追加時）

### 1. HttpManagerIs**.h を作成

```cpp
#ifndef HTTP_MANAGER_IS**_H
#define HTTP_MANAGER_IS**_H

#include "AraneaWebUI.h"

class HttpManagerIs** : public AraneaWebUI {
public:
  void begin(SettingManager* settings, int port = 80) override;

protected:
  // 機種固有タブ
  String generateTypeSpecificTabs() override;
  String generateTypeSpecificTabContents() override;
  String generateTypeSpecificJS() override;

  // 機種固有API
  void registerTypeSpecificEndpoints() override;

  // 機種固有ステータス
  void getTypeSpecificStatus(JsonObject& obj) override;
  void getTypeSpecificConfig(JsonObject& obj) override;

  // SpeedDial対応
  String generateTypeSpecificSpeedDial() override;
  bool applyTypeSpecificSpeedDial(const String& section,
                                   const std::vector<String>& lines) override;
};

#endif
```

### 2. 機種固有タブを実装

```cpp
String HttpManagerIs**::generateTypeSpecificTabs() {
  return R"TABS(
<div class="tab" data-tab="custom" onclick="showTab('custom')">Custom</div>
)TABS";
}

String HttpManagerIs**::generateTypeSpecificTabContents() {
  return R"HTML(
<div id="tab-custom" class="tab-content">
  <div class="card">
    <div class="card-title">Custom Settings</div>
    <!-- 機種固有のUI -->
  </div>
</div>
)HTML";
}
```

### 3. メインスケッチで初期化

```cpp
#include "HttpManagerIs**.h"

HttpManagerIs** webUI;

void setup() {
  // ... 他の初期化 ...

  webUI.begin(&settingManager, 80);

  // デバイス情報設定
  AraneaDeviceInfo info;
  info.deviceType = "ar-is**";
  info.modelName = "Custom Device";
  // ... 他の情報 ...
  webUI.setDeviceInfo(info);
}

void loop() {
  webUI.handle();
}
```

---

## バージョン履歴

| Version | Date | Changes |
|---------|------|---------|
| 1.6.0 | 2025-12-27 | WiFiスキャンUI追加、チップ温度表示追加 |
| 1.5.0 | 2025-12-24 | SpeedDial/WiFi API共通化 |
| 1.4.0 | 2025-12-20 | CIC認証実装 |
| 1.3.0 | 2025-12-18 | 基本タブ構成確立 |

---

## ファイル配置

```
code/ESP32/
├── global/
│   ├── src/
│   │   ├── AraneaWebUI.cpp    # 共通UIフレームワーク (68KB)
│   │   ├── AraneaWebUI.h      # 共通UIヘッダー
│   │   ├── SettingManager.cpp # 設定管理（必須依存）
│   │   └── SettingManager.h
│   └── library.properties
├── is02a/
│   ├── HttpManagerIs02a.cpp   # is02a固有UI
│   └── HttpManagerIs02a.h
├── is04a/
│   ├── HttpManagerIs04a.cpp   # is04a固有UI
│   └── HttpManagerIs04a.h
├── is05a/
│   ├── HttpManagerIs05a.cpp   # is05a固有UI
│   └── HttpManagerIs05a.h
├── is06a/
│   ├── HttpManagerIs06a.cpp   # is06a固有UI
│   └── HttpManagerIs06a.h
├── is10/
│   ├── HttpManagerIs10.cpp    # is10固有UI (最新リファレンス実装)
│   └── HttpManagerIs10.h
└── is10t/
    ├── HttpManagerIs10t.cpp   # is10t固有UI
    └── HttpManagerIs10t.h
```

---

## 注意事項

1. **カラーテーマ変更禁止**: MUI 7系の統一デザインを維持
2. **共通タブの改変禁止**: 共通タブはAraneaWebUI.cppでのみ管理
3. **依存モジュール**: SettingManagerは必須。WiFi/SPIFFS/ArduinoJsonも必要
4. **バージョン管理**: UIバージョンは`ARANEA_UI_VERSION`で一元管理
5. **機種固有タブ**: 必ず`generateTypeSpecific*`メソッドをオーバーライド
