# araneaDevice プロビジョニングフロー設計

## 概要

araneaDeviceは2つのプロビジョニングモードをサポートする:
1. **大量生産モード** - 施設固有の初期設定をファームウェアに組み込み
2. **汎用モード** - APモードのWeb UIから設定

---

## 1. 大量生産モード

### ユースケース
- 特定施設向けに多数のデバイスを製造
- 同一テナント・同一設定のデバイスを効率的にインストール

### フロー
```
1. AraneaSettings.h を施設用に編集
   - TID, FID
   - テナントプライマリ認証情報
   - WiFi設定（cluster1-6）

2. ファームウェアをビルド

3. 各デバイスにフラッシュ
   - 起動時に自動でAraneaDeviceGateへ登録
   - CIC取得 → 運用開始
```

### AraneaSettings.h 編集例
```cpp
// 施設A向け設定
#define ARANEA_DEFAULT_TID "T施設AのテナントID"
#define ARANEA_DEFAULT_FID "施設AのFID"
#define ARANEA_DEFAULT_TENANT_LACISID "テナントプライマリのlacisId"
#define ARANEA_DEFAULT_TENANT_EMAIL "テナントプライマリのEmail"
#define ARANEA_DEFAULT_TENANT_CIC "テナントプライマリのCIC"

#define ARANEA_DEFAULT_WIFI_SSID_1 "施設AのSSID1"
#define ARANEA_DEFAULT_WIFI_PASS "施設AのWiFiパスワード"
```

### メリット
- 設定作業不要で即座に運用開始
- ヒューマンエラーの削減
- 大量展開の効率化

---

## 2. 汎用モード

### ユースケース
- 汎用デバイスとして販売/配布
- 購入者が自分のテナントに登録
- 設定変更が必要な場合

### フロー
```
1. デバイス起動（初期状態 or リセット後）

2. WiFi未設定/接続失敗 → APモード起動
   - SSID: ar-is10-XXXXXX (MACアドレス末尾6桁)
   - Password: なし or 固定値

3. ユーザーがAPに接続

4. ブラウザで設定画面にアクセス
   - http://192.168.4.1/

5. 設定項目を入力:
   - WiFi設定（SSID/パスワード × 6組）
   - テナントID (TID)
   - 施設ID (FID)
   - テナントプライマリ認証情報
     - lacisId
     - Email
     - CIC

6. 保存 → デバイス再起動

7. WiFi接続 → AraneaDeviceGate登録 → 運用開始
```

### 設定画面の必須項目

| カテゴリ | 項目 | 説明 |
|----------|------|------|
| WiFi | SSID (1-6) | 接続先WiFiのSSID |
| WiFi | Password (1-6) | WiFiパスワード |
| テナント | TID | テナントID (T + 19桁) |
| テナント | FID | 施設ID |
| 認証 | テナントプライマリlacisId | 認証に使用するlacisId |
| 認証 | テナントプライマリEmail | 認証Email |
| 認証 | テナントプライマリCIC | 認証CIC |

---

## 3. 設定の優先順位

設定値の読み込み優先順位:

```
1. SettingManager (SPIFFS/NVS保存値) - 最優先
   ↓ なければ
2. AraneaSettings.h のデフォルト値 - フォールバック
```

### コード例
```cpp
// SettingManager → AraneaSettings の順でフォールバック
myTid = settings.getString("tid", ARANEA_DEFAULT_TID);
myFid = settings.getString("fid", ARANEA_DEFAULT_FID);

TenantPrimaryAuth tenantAuth;
tenantAuth.lacisId = settings.getString("tenant_lacisid", ARANEA_DEFAULT_TENANT_LACISID);
tenantAuth.userId = settings.getString("tenant_email", ARANEA_DEFAULT_TENANT_EMAIL);
tenantAuth.cic = settings.getString("tenant_cic", ARANEA_DEFAULT_TENANT_CIC);
```

---

## 4. APモード起動条件

APモードに入る条件:

| 条件 | 説明 |
|------|------|
| WiFi未設定 | SSID/パスワードが空 |
| WiFi接続失敗 | 全6組のSSIDへの接続が失敗（リトライ上限到達） |
| ボタン長押し | WiFiボタン3秒長押し |
| ファクトリーリセット | リセットボタン長押し後 |

---

## 5. AraneaSettings.h の配置

各デバイスタイプごとにAraneaSettings.hを配置:

```
generic/ESP32/
├── global/src/
│   └── AraneaSettings.h    # 共通テンプレート（ISMS用デフォルト）
│   └── AraneaSettings.cpp
├── is10/
│   └── AraneaSettings.h    # is10専用（必要に応じて上書き）
├── is04/
│   └── AraneaSettings.h    # is04専用
└── ...
```

### 大量生産時
1. `generic/ESP32/is10/AraneaSettings.h` を施設用に編集
2. ビルド & フラッシュ

### 汎用販売時
1. AraneaSettings.h は空またはプレースホルダー値
2. ユーザーがAPモードで設定

---

## 6. 設定リセット

### ファクトリーリセット
リセットボタン長押し（3秒）で実行:
```cpp
araneaReg.clearRegistration();  // 登録情報クリア
settings.clear();               // 全設定クリア
SPIFFS.remove("/is10_routers.json");
SPIFFS.remove("/aranea_settings.json");
ESP.restart();
```

リセット後:
- AraneaSettings.hのデフォルト値に戻る
- WiFi接続失敗 → APモード起動
- 再設定が必要

---

## 7. セキュリティ考慮事項

### テナントプライマリ認証情報
- **大量生産モード**: ファームウェアに埋め込み（物理アクセス必要）
- **汎用モード**: APモードはローカルネットワークのみ（インターネット非公開）

### APモードのセキュリティ
- タイムアウト設定（デフォルト5分）
- 設定完了後は自動でAPモード終了
- パスワード保護（オプション）

---

## 更新履歴

| 日付 | 内容 |
|------|------|
| 2025-12-18 | 初版作成 - プロビジョニングフロー設計 |
