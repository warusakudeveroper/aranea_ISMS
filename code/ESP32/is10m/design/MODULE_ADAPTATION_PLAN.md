# is10m モジュール転用・ボトルネック分析計画書

**更新日**: 2025/12/22 - API仕様確定後の最終更新

## 1. 潜在的ボトルネック分析

### 1.1 ✅ ログイン仕様（P0 → 解決済み）

**確定した仕様:**
- **認証方式**: XORベースのsecurityEncode（MD5/RSAではない）
- **エンドポイント**: `POST /`
- **リクエスト**: `{"method":"do","login":{...}}`
- **パスワードエンコード**: 固定キーによるXOR変換

```cpp
// パスワードエンコード実装（確定版）
String securityEncode(const String& password, const String& key1, const String& key2) {
    String result = "";
    int g = password.length();
    int m = key1.length();
    int k = key2.length();
    int h = max(g, m);

    for (int f = 0; f < h; f++) {
        int l = 187, t = 187;
        if (f >= g) t = key1[f];
        else if (f >= m) l = password[f];
        else { l = password[f]; t = key1[f]; }
        result += key2[(l ^ t) % k];
    }
    return result;
}
```

**リスク: 解決済み** - 実装・テスト済み

### 1.2 ✅ データ取得方式（HTMLパース不要）

**当初の懸念**: HTMLスクレイピングによるメモリ・パース性能

**解決策**: JSON APIが使用可能と判明
- `filter:[{group_id:["-1"]}]` パラメータが必須
- HTMLより軽量（15KB vs 230KB）
- ArduinoJsonで安全にパース可能

```cpp
// JSON APIでのデータ取得（確定版）
DynamicJsonDocument doc(16384);
String request = "{\"method\":\"get\",\"apmng_client\":{\"table\":\"client_list\",\"filter\":[{\"group_id\":[\"-1\"]}],\"para\":{\"start\":0,\"end\":99}}}";
http.POST(request);
deserializeJson(doc, http.getString());
```

**リスク: 解決済み** - JSON API方式で実装

### 1.3 VPN経由HTTP接続

| 懸念 | 詳細 | 対策 |
|------|------|------|
| **接続遅延** | VPN経由で100-500ms追加遅延 | タイムアウト30秒に設定済み |
| **接続断** | VPN不安定時の切断 | リトライ3回、次サイクルで再試行 |
| **DNS解決** | IPアドレス直接指定で回避 | 設計済み |

**リスク: 中** - VPN品質依存だが12分間隔なので影響小

### 1.4 大量クライアント対応

| シナリオ | クライアント数 | HTML推定サイズ | 処理時間推定 |
|----------|---------------|---------------|-------------|
| 通常運用 | 20-50 | 50-100KB | 1-2秒 |
| ピーク時 | 100-200 | 100-200KB | 2-4秒 |
| 最大想定 | 500 | 300-400KB | 5-10秒 |

**リスク: 低** - ストリームパースなのでメモリは問題なし、時間も許容範囲

### 1.5 マルチエンドポイント送信

| 懸念 | 詳細 | 対策 |
|------|------|------|
| **逐次送信による時間** | 3サブネット × 2-3秒 = 6-9秒 | 許容範囲（12分間隔） |
| **一部失敗時の処理** | サブネットA成功、B失敗 | 個別リトライ、失敗ログ記録 |
| **送信順序** | 依存関係なし | 設定順に逐次送信 |

**リスク: 低** - 並列化不要、逐次で十分

### 1.6 総合リスク評価（2025/12/22 更新）

| ボトルネック候補 | リスク | 優先度 | 備考 |
|-----------------|--------|--------|------|
| ~~ログイン方式~~ | ✅解決 | ~~P0~~ | XORエンコード方式確定 |
| ~~HTMLパース~~ | ✅解決 | ~~P2~~ | JSON API使用でパース不要 |
| ~~暗号化方式~~ | ✅解決 | ~~P2~~ | securityEncode実装完了 |
| VPN遅延 | 低 | P3 | タイムアウト30秒で十分 |
| 大量クライアント | 低 | P3 | JSON APIで軽量 |
| マルチ送信 | 低 | P3 | 逐次で問題なし |

**結論**: 主要ボトルネック（P0）は全て解決。実装に進める状態。

---

## 2. モジュール転用計画

### 2.1 モジュール分類

```
┌─────────────────────────────────────────────────────────────┐
│                    global/src/ 共通モジュール                 │
│  （そのまま使用 - 変更なし）                                  │
├─────────────────────────────────────────────────────────────┤
│ SettingManager, WiFiManager, NtpManager, LacisIDGenerator   │
│ AraneaRegister, DisplayManager, Operator, MqttManager       │
│ StateReporter, AraneaWebUI, HttpOtaManager, OtaManager      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    is10 → is10m 転用モジュール                │
├─────────────────────────────────────────────────────────────┤
│ AraneaSettings.h/cpp   → AraneaSettings.h/cpp (大幅修正)    │
│ Is10Keys.h             → Is10mKeys.h (大幅修正)             │
│ HttpManagerIs10.h/cpp  → HttpManagerIs10m.h/cpp (大幅修正)  │
│ StateReporterIs10.h/cpp→ StateReporterIs10m.h/cpp (修正)   │
│ CelestialSenderIs10.h/cpp→ CelestialSenderIs10m.h/cpp (修正)│
│ MqttConfigHandler.h/cpp→ MqttConfigHandler.h/cpp (軽微修正) │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    is10m 新規モジュール                       │
├─────────────────────────────────────────────────────────────┤
│ MercuryTypes.h         (新規)                               │
│ MercuryACScraper.h/cpp (新規)                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    不要（削除）モジュール                      │
├─────────────────────────────────────────────────────────────┤
│ SshPollerIs10.h/cpp    → 削除（MercuryACScraperで置換）     │
│ RouterTypes.h          → 削除（MercuryTypesで置換）         │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 global/src/ 共通モジュール使用一覧

| モジュール | 使用 | 用途 |
|-----------|------|------|
| SettingManager | ○ | NVS永続化 |
| WiFiManager | ○ | WiFi接続（cluster1-6） |
| NtpManager | ○ | 時刻同期 |
| LacisIDGenerator | ○ | デバイスLacisID生成 |
| AraneaRegister | ○ | デバイス登録・CIC取得 |
| DisplayManager | ○ | OLED表示 |
| Operator | ○ | 状態管理 |
| MqttManager | ○ | MQTT WebSocket |
| StateReporter | ○ | deviceStateReport送信 |
| AraneaWebUI | ○ | Web UI基底クラス |
| HttpOtaManager | ○ | HTTP OTA（推奨） |
| OtaManager | △ | ArduinoOTA（mDNS、VPN環境では無効化） |
| SshClient | ✕ | 不要 |
| BleIsmsParser | ✕ | 不要 |
| HttpRelayClient | ✕ | 不要 |
| RebootScheduler | △ | 必要に応じて |

---

### 2.3 各モジュール転用詳細

#### 2.3.1 Is10Keys.h → Is10mKeys.h

**変更内容:** キー名変更、SSH関連削除、Mercury関連追加

```cpp
// Is10mKeys.h

#ifndef IS10M_KEYS_H
#define IS10M_KEYS_H

#define NVS_KEY(name, value) \
  static constexpr const char name[] = value; \
  static_assert(sizeof(value) - 1 <= 15, "NVS key '" value "' exceeds 15 chars")

namespace Is10mKeys {

  // --- Mercury AC設定 ---
  NVS_KEY(kAcHost,      "is10m_ac_host");   // 14文字: ACホスト
  NVS_KEY(kAcPort,      "is10m_ac_port");   // 14文字: ACポート
  NVS_KEY(kAcUser,      "is10m_ac_user");   // 14文字: ACユーザー名
  NVS_KEY(kAcPass,      "is10m_ac_pass");   // 14文字: ACパスワード
  NVS_KEY(kAcEnabled,   "is10m_ac_en");     // 11文字: AC有効フラグ

  // --- ポーリング設定 ---
  NVS_KEY(kPollIntv,    "is10m_poll_iv");   // 13文字: ポーリング間隔(ms)
  NVS_KEY(kHttpTimeout, "is10m_http_to");   // 13文字: HTTPタイムアウト(ms)

  // --- エンドポイント設定（SPIFFS /mercury_config.json で管理）---
  // サブネット別設定は複雑なためSPIFFS JSON推奨

  // --- SSOT設定 ---
  NVS_KEY(kSchema,      "is10m_schema");    // 12文字: schemaVersion
  NVS_KEY(kHash,        "is10m_hash");      // 10文字: configHash
  NVS_KEY(kAppliedAt,   "is10m_applied");   // 13文字: 最終適用日時
  NVS_KEY(kApplyErr,    "is10m_aperr");     // 11文字: 適用エラー

}  // namespace Is10mKeys

// CommonKeysはis10から流用（変更なし）
namespace CommonKeys {
  // ... is10と同一
}

#endif // IS10M_KEYS_H
```

**修正規模:** 中（キー名置換、不要削除）

---

#### 2.3.2 AraneaSettings.h/cpp

**変更内容:** デフォルト値変更、Mercury AC設定追加

```cpp
// AraneaSettings.h 主要変更点

// デバイス識別
#define ARANEA_DEVICE_TYPE     "ar-is10m"
#define ARANEA_MODEL_NAME      "Mercury_AC_Monitor"
#define ARANEA_PRODUCT_TYPE    "010"
#define ARANEA_PRODUCT_CODE    "0002"  // is10=0001, is10m=0002

// Mercury ACデフォルト設定
#define MERCURY_DEFAULT_HOST   "192.168.96.5"
#define MERCURY_DEFAULT_PORT   80
#define MERCURY_DEFAULT_USER   "isms"
#define MERCURY_DEFAULT_PASS   "isms12345@"

// タイミングデフォルト
#define MERCURY_POLL_INTERVAL_MS  720000   // 12分
#define MERCURY_HTTP_TIMEOUT_MS   30000    // 30秒
```

**修正規模:** 小（定数変更のみ）

---

#### 2.3.3 HttpManagerIs10m.h/cpp

**変更内容:** ルーター設定UI削除、Mercury AC設定UI追加、サブネット設定UI追加

**is10との差分:**

| 機能 | is10 | is10m |
|------|------|-------|
| Global Settings | SSH Timeout, Retry等 | HTTP Timeout, Poll Interval |
| Routers タブ | 最大20台のルーター設定 | **削除** |
| Mercury AC タブ | なし | **新規追加** |
| Subnets タブ | なし | **新規追加**（マルチエンドポイント） |
| LacisID Gen タブ | ルーター用 | **削除または簡略化** |
| Tenant タブ | そのまま | 流用 |
| WiFi タブ | そのまま | 流用 |
| System タブ | そのまま | 流用 |

**新規タブ: Mercury AC設定**
```html
<form>
  <label>Host:</label> <input name="host" value="192.168.96.5">
  <label>Port:</label> <input name="port" value="80">
  <label>Username:</label> <input name="username">
  <label>Password:</label> <input name="password" type="password">
  <label>Enabled:</label> <input name="enabled" type="checkbox">
</form>
```

**新規タブ: Subnets設定（エンドポイント振り分け）**
```html
<table>
  <tr><th>Subnet</th><th>Endpoint URL</th><th>TID</th><th>FID</th><th>Enabled</th></tr>
  <tr>
    <td><input name="subnet[]" value="192.168.97"></td>
    <td><input name="endpoint[]" value="https://..."></td>
    <td><input name="tid[]"></td>
    <td><input name="fid[]"></td>
    <td><input name="enabled[]" type="checkbox"></td>
  </tr>
  <!-- 最大10行 -->
</table>
```

**修正規模:** 大（UI大幅変更、APIエンドポイント追加）

---

#### 2.3.4 StateReporterIs10m.h/cpp

**変更内容:** SshPoller → MercuryACScraper 参照変更、ペイロード構造変更

**is10との差分:**

```cpp
// is10: SshPollerIs10への依存
void begin(..., SshPollerIs10* sshPoller, RouterConfig* routers, int* routerCount);

// is10m: MercuryACScraperへの依存
void begin(..., MercuryACScraper* scraper);
```

**buildPayload()変更点:**
- `routers[]`配列 → `aps[]`配列
- `routerMac`, `wanIp` → `apMac`, `apIp`
- `sshPoller->getRouterInfos()` → `scraper->getAPs()`

**修正規模:** 中（依存変更、JSON構造変更）

---

#### 2.3.5 CelestialSenderIs10m.h/cpp

**変更内容:** マルチエンドポイント対応、クライアント情報送信追加

**is10との差分:**

```cpp
// is10: 単一エンドポイント
bool send();

// is10m: サブネット別送信
bool sendForSubnet(const SubnetEndpoint& endpoint,
                   MercuryClient* clients, int clientCount);
bool sendAll();  // 全サブネット一括送信
```

**JSONペイロード変更:**
```json
// is10: devices[]配列（ルーター情報）
{
  "auth": {...},
  "devices": [{"mac": "...", "type": "Router", ...}]
}

// is10m: clients[]配列（クライアント情報）
{
  "auth": {...},
  "observedAt": 1703232000000,
  "source": "ar-is10m",
  "mercuryAc": {
    "host": "192.168.96.5",
    "apCount": 18,
    "clientCount": 12
  },
  "clients": [
    {"mac": "6C-C8-40-8C-CD-4C", "apName": "ap02-10", "ip": "192.168.97.15", ...}
  ],
  "aps": [
    {"name": "ap02-10", "mac": "4C-B7-E0-59-FC-EF", "ip": "192.168.96.19", ...}
  ]
}
```

**修正規模:** 大（マルチエンドポイント対応、JSON構造変更）

---

#### 2.3.6 MqttConfigHandler.h/cpp

**変更内容:** キー名変更、設定項目変更

**is10との差分:**
- `RouterConfig` → `SubnetEndpoint`
- `sshTimeout` → `httpTimeout`
- `routerInterval` → `pollInterval`

**修正規模:** 小〜中（設定項目の読み替え）

---

### 2.4 AraneaGlobalImporter.h (is10m版)

```cpp
// AraneaGlobalImporter.h for is10m

#ifndef ARANEA_GLOBAL_IMPORTER_H
#define ARANEA_GLOBAL_IMPORTER_H

// --- 必須モジュール ---
#include "SettingManager.h"
#include "WiFiManager.h"
#include "NtpManager.h"
#include "LacisIDGenerator.h"
#include "AraneaRegister.h"
#include "DisplayManager.h"
#include "Operator.h"

// --- 通信モジュール ---
#include "MqttManager.h"
#include "StateReporter.h"
#include "AraneaWebUI.h"

// --- OTA更新モジュール ---
// #include "OtaManager.h"       // VPN環境ではmDNS使用不可、無効化
#include "HttpOtaManager.h"      // HTTP経由版（推奨）

// --- 不要モジュール（is10mでは使用しない）---
// #include "SshClient.h"        // SSH不要
// #include "BleIsmsParser.h"    // BLE不要
// #include "HttpRelayClient.h"  // Zero3中継不要

// ============================================================
// IS10M固有モジュール
// ============================================================
#include "Is10mKeys.h"           // NVSキー定数
#include "AraneaSettings.h"      // 施設設定
#include "MercuryTypes.h"        // Mercury AC型定義
#include "MercuryACScraper.h"    // Mercury ACスクレイパー
#include "HttpManagerIs10m.h"    // Web UI実装
#include "StateReporterIs10m.h"  // deviceStateReportペイロード
#include "CelestialSenderIs10m.h"// CelestialGlobe送信
#include "MqttConfigHandler.h"   // MQTT config適用

#endif // ARANEA_GLOBAL_IMPORTER_H
```

---

## 3. ファイル構成（最終）

```
is10m/
├── is10m.ino                    # メインスケッチ
├── AraneaGlobalImporter.h       # モジュールインポート管理
│
├── # 新規モジュール
├── MercuryTypes.h               # データ型定義
├── MercuryACScraper.h           # スクレイパー
├── MercuryACScraper.cpp
│
├── # is10から転用（大幅修正）
├── Is10mKeys.h                  # NVSキー定数
├── AraneaSettings.h             # デフォルト設定
├── AraneaSettings.cpp
├── HttpManagerIs10m.h           # Web UI
├── HttpManagerIs10m.cpp
├── StateReporterIs10m.h         # State Reporter
├── StateReporterIs10m.cpp
├── CelestialSenderIs10m.h       # CelestialGlobe送信
├── CelestialSenderIs10m.cpp
├── MqttConfigHandler.h          # MQTT config (軽微修正)
├── MqttConfigHandler.cpp
│
└── design/
    ├── DESIGN.md                # 全体設計書
    ├── MercuryACScraper_PLAN.md # スクレイパー設計
    └── MODULE_ADAPTATION_PLAN.md # このファイル
```

---

## 4. 実装優先順位

### Phase 1: コア機能（P0）
1. MercuryTypes.h 作成
2. MercuryACScraper.h/cpp 実装
   - ログイン機能
   - ストリームパーサー
3. Is10mKeys.h 作成
4. AraneaSettings.h/cpp 修正

### Phase 2: 送信機能（P1）
5. CelestialSenderIs10m.h/cpp 実装
6. StateReporterIs10m.h/cpp 実装

### Phase 3: UI・統合（P2）
7. HttpManagerIs10m.h/cpp 実装
8. MqttConfigHandler.h/cpp 修正
9. is10m.ino 作成

### Phase 4: テスト（P3）
10. 単体テスト
11. 統合テスト
12. 長時間稼働テスト

---

## 5. リスク・注意事項

### 5.1 P0: ログイン仕様確認必須

Mercury ACのログインAPIは**実機確認必須**:
- MD5ハッシュのフォーマット
- POSTボディの正確な形式
- トークン取得方法

**事前検証方法:**
```bash
# 開発者ツールでNetwork監視しながらログイン
# または
curl -X POST http://192.168.96.5/ \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=isms&password=XXXXX"
```

### 5.2 VPN環境での注意

- mDNS（ArduinoOTA）は**使用不可** → HttpOtaManagerのみ使用
- DNS解決よりIPアドレス直接指定を推奨
- 接続タイムアウトは長め（30秒）に設定

### 5.3 min_spiffsパーティション

- 必ず`min_spiffs`を使用（OTA領域確保）
- `huge_app`は**絶対禁止**
- ビルドオプション: `esp32:esp32:esp32:PartitionScheme=min_spiffs`
