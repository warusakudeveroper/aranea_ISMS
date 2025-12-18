# araneaDeviceGate 認証仕様

## 概要

ESP32デバイスがmobes2.0クラウドに登録する際の認証仕様。
2024年12月以降、認証方式が変更され **passフィールドは廃止** されました。

## 認証の3要素

| 要素 | 説明 | 例 |
|------|------|-----|
| `lacisId` | テナントプライマリのlacisId | `18217487937895888001` |
| `userId` | テナントプライマリのメールアドレス | `soejim@mijeos.com` |
| `cic` | テナントプライマリのCIC（6桁） | `204965` |

> **重要**: `pass` フィールドは送信しても無視されます。認証は上記3要素のみで行われます。

## 登録リクエスト形式

### エンドポイント

```
POST https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate
Content-Type: application/json
```

### リクエストボディ

```json
{
  "lacisOath": {
    "lacisId": "18217487937895888001",
    "userId": "soejim@mijeos.com",
    "cic": "204965",
    "method": "register"
  },
  "userObject": {
    "lacisID": "301030C92212F6800001",
    "tid": "T2025120621041161827",
    "typeDomain": "araneaDevice",
    "type": "ar-is10"
  },
  "deviceMeta": {
    "macAddress": "30C92212F680",
    "productType": "010",
    "productCode": "0001"
  }
}
```

### レスポンス（成功時）

```json
{
  "ok": true,
  "existing": false,
  "lacisId": "301030C92212F6800001",
  "userObject": {
    "cic_code": "696640",
    "cic_active": true,
    "permission": 10
  },
  "stateEndpoint": "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport"
}
```

- `existing: true` の場合、MACアドレスで既存デバイスが認識された
- `cic_code` がデバイスに発行されたCIC（NVSに保存して使用）

## テナント情報（プロジェクト別）

### IS10 (Openwrt_RouterInspector)

```cpp
// AraneaSettings.h
#define ARANEA_DEFAULT_TID              "T2025120621041161827"
#define ARANEA_DEFAULT_FID              "0150"
#define ARANEA_DEFAULT_TENANT_LACISID   "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL     "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC       "204965"
// TENANT_PASS は廃止
```

### ISMS (市山水産株式会社)

```cpp
// AraneaSettings.h
#define ARANEA_DEFAULT_TID              "T2025120608261484221"
#define ARANEA_DEFAULT_FID              "9000"
#define ARANEA_DEFAULT_TENANT_LACISID   "12767487939173857894"
#define ARANEA_DEFAULT_TENANT_EMAIL     "info+ichiyama@neki.tech"
#define ARANEA_DEFAULT_TENANT_CIC       "263238"
// TENANT_PASS は廃止
```

## NVS名前空間

ESP32のNVS（Non-Volatile Storage）は以下の名前空間を使用：

| 名前空間 | 用途 | クリア方法 |
|---------|------|-----------|
| `isms` | デバイス設定（WiFi, relay等） | `SettingManager::clear()` |
| `aranea` | 登録データ（CIC, stateEndpoint） | `AraneaRegister::clearRegistration()` |

### ファクトリーリセット時の注意

`handleFactoryReset()` では **両方の名前空間** をクリアする必要があります：

```cpp
void HttpManagerIs10::handleFactoryReset() {
  // 1. "isms" namespace を clear
  settings_->clear();

  // 2. "aranea" namespace を clear（重要！）
  Preferences araneaPrefs;
  araneaPrefs.begin("aranea", false);
  araneaPrefs.clear();
  araneaPrefs.end();

  // 3. SPIFFSファイル削除
  SPIFFS.remove("/routers.json");
  SPIFFS.remove("/aranea_settings.json");

  ESP.restart();
}
```

## TenantPrimaryAuth 構造体

```cpp
// AraneaRegister.h
struct TenantPrimaryAuth {
  String lacisId;
  String userId;
  String cic;
  // passは廃止（認証はlacisId + userId + cicの3要素）
};
```

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2024-12-18 | passフィールド廃止、3要素認証に移行 |
| 2024-12-18 | ファクトリーリセットで"aranea"名前空間もクリアするよう修正 |

## 関連ファイル

- `AraneaRegister.h` / `AraneaRegister.cpp` - 登録処理
- `AraneaSettings.h` / `AraneaSettings.cpp` - デフォルト設定
- `SettingManager.h` / `SettingManager.cpp` - NVS設定管理
- `HttpManagerIs10.cpp` - ファクトリーリセット処理
