# AraneaWebUI v1.5 ESP32版 設計ドキュメント

## 概要

AraneaWebUI v1.5は、AraneaデバイスのESP32版共通Webインターフェースです。
is20s（Orange Pi/Python版）で実装された管理UIの共通部分をESP32に完全移植し、
全Araneaデバイスで統一されたUI/UXを提供します。

### バージョン履歴

| Version | Date | Description |
|---------|------|-------------|
| 1.0.0 | 2024-12 | 初期版: 基本タブ構成（Status/Network/Cloud/Tenant/System） |
| 1.5.0 | 2025-01 | is20s共通UIの完全移植、SpeedDial、WiFiマルチ設定、APモード初期設定フロー |

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    AraneaWebUI (Base Class)                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  共通コンポーネント（全デバイス共通）                  │  │
│  │  - Status / Network / Cloud / Tenant / System / SpeedDial│  │
│  │  - APモード初期設定フロー                              │  │
│  │  - 認証（Basic Auth / CIC）                           │  │
│  │  - 共通CSS/JS/SVG                                     │  │
│  └───────────────────────────────────────────────────────┘  │
│                           ↓ 継承                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  デバイス固有コンポーネント（派生クラスで実装）        │  │
│  │  - is04a: Channels / Triggers                         │  │
│  │  - is05a: Rules / Channels                            │  │
│  │  - is10:  Inspector / Routers                         │  │
│  │  - is10t: Cameras / ONVIF                             │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 共通タブ構成

### v1.5 タブ一覧

| Tab | 共通/固有 | Description |
|-----|----------|-------------|
| **Status** | 共通 | デバイス情報、ネットワーク状態、ライブステータス |
| **Network** | 共通 | WiFi設定（6スロット）、NTP設定 |
| **Cloud** | 共通 | AraneaDeviceGate、Relay Endpoints |
| **Tenant** | 共通 | テナント認証（TID/FID/LacisID/CIC） |
| **System** | 共通 | デバイス名、UIパスワード、定期再起動、Factory Reset |
| **SpeedDial** | 共通 | 設定一括管理（INIテキスト形式） |
| *Device-specific* | 固有 | 各デバイス固有タブ |

---

## 共通コンポーネント詳細

### 1. Status Tab

#### 1.1 Device Information カード

```
┌─────────────────────────────────────────┐
│ Device Information                      │
├─────────────────────────────────────────┤
│ Type      │ ar-is10                     │
│ LacisID   │ 30200200AABBCCDDEEFF0001    │
│ CIC       │ 263238                      │
│ MAC       │ AA:BB:CC:DD:EE:FF           │
│ Hostname  │ is10-router-mon             │
│ Version   │ 1.2.0 / UI 1.5.0            │
└─────────────────────────────────────────┘
```

#### 1.2 Live Status カード

```
┌─────────────────────────────────────────┐
│ Live Status (5秒間隔で自動更新)         │
├─────────────────────────────────────────┤
│ IP Address │ 192.168.3.100             │
│ SSID       │ cluster1                   │
│ RSSI       │ -65 dBm                    │
│ NTP Time   │ 2025-01-15T10:30:00Z      │
│ Uptime     │ 3d 5h 20m                  │
│ Free Heap  │ 120.5 KB                   │
└─────────────────────────────────────────┘
```

**ESP32制約**: Heapは`ESP.getFreeHeap()`を使用。80KB以下で警告表示（黄色）、50KB以下で危険表示（赤）。

---

### 2. Network Tab

#### 2.1 WiFi接続状態カード

```
┌─────────────────────────────────────────────────────────────┐
│ WiFi接続状態                                                 │
├─────────────────────────────────────────────────────────────┤
│ STATUS │ Connected    │ SSID   │ cluster1                   │
│ SIGNAL │ -65 dBm      │ IP     │ 192.168.3.100              │
├─────────────────────────────────────────────────────────────┤
│ 一時接続（設定リストに保存しません）                         │
│ ┌─────────────────┐ ┌─────────────────┐                     │
│ │ SSID            │ │ Password        │  [一時接続] [Scan]  │
│ └─────────────────┘ └─────────────────┘                     │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 WiFi設定一覧カード（6スロット）

```
┌─────────────────────────────────────────────────────────────┐
│ WiFi設定一覧 (最大6件・優先順位順)                          │
├─────────────────────────────────────────────────────────────┤
│ 保存されたWiFi設定を上から順に接続を試行します。            │
│ 遠隔地でのAP交換時も旧SSIDを残しておけば切り替え可能です。  │
├──────┬─────────────────────┬──────────┬────────────────────┤
│  #   │ SSID                │ Password │ 操作               │
├──────┼─────────────────────┼──────────┼────────────────────┤
│  1   │ cluster1            │ ******** │ [↑][↓][🗑️][接続]  │
│  2   │ cluster2            │ ******** │ [↑][↓][🗑️][接続]  │
│  3   │ cluster3            │ ******** │ [↑][↓][🗑️][接続]  │
│  4   │ (empty)             │          │                    │
│  5   │ (empty)             │          │                    │
│  6   │ (empty)             │          │                    │
├──────┴─────────────────────┴──────────┴────────────────────┤
│ [Auto Connect] [Reset to Default] [Refresh]                │
├─────────────────────────────────────────────────────────────┤
│ 新しいWiFi設定をリストに追加                                │
│ ┌─────────────────┐ ┌─────────────────┐                     │
│ │ SSID            │ │ Password        │  [リストに追加]     │
│ └─────────────────┘ └─────────────────┘                     │
└─────────────────────────────────────────────────────────────┘
```

**API設計**:
```
GET  /api/wifi/list         - WiFi設定一覧取得
POST /api/wifi/add          - 設定追加 {ssid, password}
POST /api/wifi/delete       - 設定削除 {index}
POST /api/wifi/move         - 順序変更 {index, direction: "up"|"down"}
POST /api/wifi/connect      - 即時接続 {index} または {ssid, password}
POST /api/wifi/scan         - スキャン実行
GET  /api/wifi/scan/result  - スキャン結果取得
POST /api/wifi/reset        - デフォルトに戻す（cluster1-6）
POST /api/wifi/auto         - 自動接続（リスト順に試行）
```

**ESP32実装**:
```cpp
// NVSストレージ
wifi_ssid1 〜 wifi_ssid6
wifi_pass1 〜 wifi_pass6

// 自動接続ロジック
for (int i = 1; i <= 6; i++) {
  String ssid = settings->getString("wifi_ssid" + String(i), "");
  if (ssid.length() == 0) continue;
  String pass = settings->getString("wifi_pass" + String(i), "");
  if (tryConnect(ssid, pass, 15000)) {  // 15秒タイムアウト
    return true;
  }
}
```

#### 2.3 NTP設定カード

```
┌─────────────────────────────────────────────────────────────┐
│ NTP / Time Settings                                         │
├─────────────────────────────────────────────────────────────┤
│ Synchronized │ Yes (green)  │ Current Time │ 2025-01-15... │
│ Server       │ ntp.nict.jp  │ Timezone     │ Asia/Tokyo    │
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────┐ ┌─────────────────┐                     │
│ │ NTP Server      │ │ Timezone        │  [Save NTP]        │
│ └─────────────────┘ └─────────────────┘                     │
└─────────────────────────────────────────────────────────────┘
```

**ESP32実装**: `configTzTime("JST-9", "ntp.nict.jp")`

---

### 3. Cloud Tab

```
┌─────────────────────────────────────────────────────────────┐
│ AraneaDeviceGate                                            │
├─────────────────────────────────────────────────────────────┤
│ Gate URL       │ https://...araneaDeviceGate               │
│ StateReport URL│ https://...deviceStateReport              │
├─────────────────────────────────────────────────────────────┤
│ Relay Endpoints (Zero3)                                     │
├─────────────────────────────────────────────────────────────┤
│ Primary   │ http://192.168.96.201:8080                      │
│ Secondary │ http://192.168.96.202:8080                      │
│                                            [Save Cloud]     │
└─────────────────────────────────────────────────────────────┘
```

---

### 4. Tenant Tab

```
┌─────────────────────────────────────────────────────────────┐
│ Tenant Authentication                                       │
├─────────────────────────────────────────────────────────────┤
│ Tenant ID (TID)   │ T2025120608261484221                    │
│ Facility ID (FID) │ 9000                                    │
├─────────────────────────────────────────────────────────────┤
│ Primary LacisID   │ 12767487939173857894                    │
│ CIC               │ 263238                                  │
├─────────────────────────────────────────────────────────────┤
│ Email             │ info+ichiyama@neki.tech                 │
│                                           [Save Tenant]     │
└─────────────────────────────────────────────────────────────┘
```

**バリデーション**:
- TID: `T` + 19桁の数字（計20文字）
- FID: 4桁の数字
- LacisID: 20桁の数字

---

### 5. System Tab

```
┌─────────────────────────────────────────────────────────────┐
│ Device Identity (LacisOath)                                 │
├─────────────────────────────────────────────────────────────┤
│ Device Name │ 本社1F-ルーター監視                           │
│ ※ この名前はMQTT/クラウドでuserobject.nameとして使用されます │
├─────────────────────────────────────────────────────────────┤
│ Security                                                    │
├─────────────────────────────────────────────────────────────┤
│ UI Password (Basic Auth) │ ********                         │
│                          │ (空欄なら認証なし)               │
├─────────────────────────────────────────────────────────────┤
│ Scheduled Reboot                                            │
├─────────────────────────────────────────────────────────────┤
│ [✓] Enable daily reboot │ Time: 03:00                       │
│                                           [Save System]     │
├─────────────────────────────────────────────────────────────┤
│ Actions                                                     │
├─────────────────────────────────────────────────────────────┤
│ [Reboot Now]  [Factory Reset]                               │
└─────────────────────────────────────────────────────────────┘
```

**Factory Reset動作**:
1. NVS `isms` namespace クリア
2. NVS `aranea` namespace クリア
3. SPIFFSの全ファイル削除
4. ESP.restart()

---

### 6. SpeedDial Tab（新規）

#### 6.1 UI設計

```
┌─────────────────────────────────────────────────────────────┐
│ SpeedDial 設定一括管理                                      │
├─────────────────────────────────────────────────────────────┤
│ 管理者から送られた設定テキストを貼り付けて一括適用できます。 │
├─────────────────────────────────────────────────────────────┤
│ 現在の設定                                     [📋 Copy]    │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ [Device]                                                │ │
│ │ name=本社1F-ルーター監視                                │ │
│ │ lacisid=30200200AABBCCDDEEFF0001                        │ │
│ │ hostname=is10-router-mon                                │ │
│ │                                                         │ │
│ │ [WiFi]                                                  │ │
│ │ wifi1=cluster1,isms12345@                               │ │
│ │ wifi2=cluster2,isms12345@                               │ │
│ │ ...                                                     │ │
│ └─────────────────────────────────────────────────────────┘ │
│ [🔄 Refresh]                                                │
├─────────────────────────────────────────────────────────────┤
│ 設定を貼り付けて適用                                        │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ ここに設定テキストを貼り付け...                         │ │
│ └─────────────────────────────────────────────────────────┘ │
│ [✅ 適用]  [🗑️ クリア]                                      │
├─────────────────────────────────────────────────────────────┤
│ SpeedDial フォーマット説明                                  │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ [WiFi]                                                  │ │
│ │ wifi1=SSID名,パスワード                                 │ │
│ │ wifi2=SSID名2,パスワード2                               │ │
│ │                                                         │ │
│ │ [NTP]                                                   │ │
│ │ server=ntp.nict.jp                                      │ │
│ │ timezone=Asia/Tokyo                                     │ │
│ │                                                         │ │
│ │ [Cloud]                                                 │ │
│ │ gate_url=https://...                                    │ │
│ │ relay_primary=http://192.168.96.201:8080                │ │
│ │                                                         │ │
│ │ ※ 変更したいセクションのみ含めれば部分更新可能         │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

#### 6.2 SpeedDial INIフォーマット

SpeedDialは**共通セクション**と**デバイス固有セクション**で構成されます。
派生クラスで `generateTypeSpecificSpeedDial()` をオーバーライドして固有セクションを追加します。

##### 共通セクション（全ESP32デバイス）

```ini
[Device]
name=デバイス名
lacisid=（読み取り専用、出力のみ）
hostname=ホスト名

[WiFi]
wifi1=SSID,パスワード
wifi2=SSID,パスワード
wifi3=SSID,パスワード
wifi4=SSID,パスワード
wifi5=SSID,パスワード
wifi6=SSID,パスワード

[NTP]
server=ntp.nict.jp
timezone=Asia/Tokyo

[Cloud]
gate_url=https://...araneaDeviceGate
state_report_url=https://...deviceStateReport
relay_primary=http://192.168.96.201:8080
relay_secondary=http://192.168.96.202:8080

[Tenant]
tid=T2025120608261484221
fid=9000
lacisid=12767487939173857894
email=info+ichiyama@neki.tech
cic=263238

[System]
device_name=本社1F-ルーター監視
reboot_enabled=true
reboot_time=03:00
```

##### デバイス固有セクション例

**is10 (Router Inspector):**
```ini
[Inspector]
endpoint=https://...universalIngest
scan_interval_sec=60
report_clients=true
ssh_timeout=30000

[Routers]
router1=306,192.168.125.186,65305,admin,password,openwrt
router2=307,192.168.125.187,65305,admin,password,asuswrt
```

**is04a (Relay Controller):**
```ini
[Triggers]
trigger1=webhook,http://example.com/notify,ch1,on
trigger2=schedule,12:00,ch2,toggle

[Channels]
ch1=名前,初期状態(on/off)
ch2=名前,初期状態(on/off)
```

**is05a (Reed Switch Monitor):**
```ini
[Rules]
rule1=ch1,open,webhook,http://example.com/alert
rule2=ch2,close,mqtt,sensors/door

[Channels]
ch1=玄関ドア,normally_closed
ch2=窓センサー,normally_open
```

#### 6.3 ESP32実装

##### AraneaWebUI基底クラス

```cpp
// 派生クラスでオーバーライド可能な仮想メソッド
class AraneaWebUI {
protected:
  /**
   * デバイス固有SpeedDialセクション生成
   * 派生クラスでオーバーライドして独自セクションを追加
   */
  virtual String generateTypeSpecificSpeedDial() { return ""; }

  /**
   * デバイス固有SpeedDialセクション適用
   * @return 適用したセクション名のリスト
   */
  virtual std::vector<String> applyTypeSpecificSpeedDial(const String& section, const std::map<String, String>& values) {
    return {};
  }
};
```

##### GET /api/speeddial 実装

```cpp
void handleSpeedDialGet() {
  String text = "";

  // === 共通セクション ===
  // [Device]
  text += "[Device]\n";
  text += "name=" + settings_->getString("device_name", "") + "\n";
  text += "lacisid=" + deviceInfo_.lacisId + "\n";
  text += "hostname=" + WiFi.getHostname() + "\n\n";

  // [WiFi]
  text += "[WiFi]\n";
  for (int i = 1; i <= 6; i++) {
    String ssid = settings_->getString(("wifi_ssid" + String(i)).c_str(), "");
    String pass = settings_->getString(("wifi_pass" + String(i)).c_str(), "");
    if (ssid.length() > 0) {
      text += "wifi" + String(i) + "=" + ssid + "," + pass + "\n";
    }
  }
  text += "\n";

  // [NTP], [Cloud], [Tenant], [System] ...

  // === デバイス固有セクション ===
  text += generateTypeSpecificSpeedDial();

  server_->send(200, "application/json", "{\"ok\":true,\"text\":\"" + escapeJson(text) + "\"}");
}
```

##### POST /api/speeddial 実装

```cpp
void handleSpeedDialSet() {
  // INIパース
  String body = server_->arg("plain");
  std::vector<String> applied;
  std::vector<String> errors;

  String currentSection = "";
  std::map<String, String> sectionValues;

  // 1行ずつ処理（メモリ節約）
  int pos = 0;
  while (pos < body.length()) {
    int nl = body.indexOf('\n', pos);
    String line = (nl >= 0) ? body.substring(pos, nl) : body.substring(pos);
    line.trim();
    pos = (nl >= 0) ? nl + 1 : body.length();

    if (line.startsWith("[") && line.endsWith("]")) {
      // 前セクション適用
      if (currentSection.length() > 0) {
        if (applySection(currentSection, sectionValues)) {
          applied.push_back(currentSection);
        }
      }
      currentSection = line.substring(1, line.length() - 1);
      sectionValues.clear();
    } else if (line.indexOf('=') > 0) {
      int eq = line.indexOf('=');
      String key = line.substring(0, eq);
      String val = line.substring(eq + 1);
      sectionValues[key] = val;
    }
  }
  // 最終セクション適用
  if (currentSection.length() > 0) {
    if (applySection(currentSection, sectionValues)) {
      applied.push_back(currentSection);
    }
  }

  // レスポンス
  // {ok:true, applied:["WiFi","NTP","Inspector"], errors:[]}
}

bool applySection(const String& section, const std::map<String, String>& values) {
  // 共通セクション
  if (section == "WiFi") { return applyWiFiSection(values); }
  if (section == "NTP") { return applyNTPSection(values); }
  if (section == "Cloud") { return applyCloudSection(values); }
  if (section == "Tenant") { return applyTenantSection(values); }
  if (section == "System") { return applySystemSection(values); }

  // デバイス固有セクション（派生クラスで処理）
  auto result = applyTypeSpecificSpeedDial(section, values);
  return !result.empty();
}
```

**ESP32制約**: INIパースは軽量実装（JSONより低メモリ）。1行ずつ処理してメモリ節約。

---

## APモード初期設定フロー

### フロー概要

```
┌──────────────────────────────────────────────────────────────────┐
│                    APモード初期設定フロー                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. 電源ON                                                       │
│     ↓                                                            │
│  2. WiFi設定確認                                                 │
│     ├─ 設定あり → WiFi接続試行（6スロット順）                    │
│     │              ├─ 成功 → 通常動作モード                      │
│     │              └─ 全失敗 → APモード起動（タイムアウト後）    │
│     │                                                            │
│     └─ 設定なし → APモード起動                                   │
│                                                                  │
│  3. APモード起動                                                 │
│     - SSID: "Aranea-{DeviceType}-{MAC下6桁}"                    │
│     - Password: なし（オープン）または "aranea123"              │
│     - IP: 192.168.4.1                                            │
│                                                                  │
│  4. ユーザーがAPに接続                                           │
│     - スマホ/PCからSSID選択・接続                                │
│                                                                  │
│  5. 初期設定画面表示 (Captive Portal)                            │
│     - http://192.168.4.1 または自動リダイレクト                  │
│                                                                  │
│  6. 設定入力（簡易ウィザード形式）                               │
│     Step 1: WiFi設定（SSID/Password）                            │
│     Step 2: テナント設定（TID/FID/Email/CIC）- オプション        │
│     Step 3: デバイス名設定 - オプション                          │
│                                                                  │
│  7. 設定保存 → WiFi接続試行                                      │
│     ├─ 成功 → APモード終了 → 通常動作モード                     │
│     └─ 失敗 → エラー表示 → Step 1に戻る                         │
│                                                                  │
│  8. 通常動作モード                                               │
│     - 通常UIにアクセス可能                                       │
│     - 設定変更は認証後のみ                                       │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### APモード専用UI

```html
<!-- Captive Portal / 初期設定画面 -->
┌─────────────────────────────────────────────────────────────┐
│              🕸️ Aranea Device Setup                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Device: ar-is10                                      │   │
│  │ MAC: AA:BB:CC:DD:EE:FF                              │   │
│  │ LacisID: 30200200AABBCCDDEEFF0001                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Step 1/3: WiFi設定                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ SSID     │ [________________] [Scan]               │   │
│  │ Password │ [________________]                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  📡 利用可能なネットワーク                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ○ cluster1       -65dBm  🔒                        │   │
│  │ ○ cluster2       -72dBm  🔒                        │   │
│  │ ○ Guest          -80dBm  🔓                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  [Skip] [← Back]                     [Next →]              │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  SpeedDial: 設定テキストを貼り付けて一括設定                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                                                     │   │
│  │  [ここに設定テキストを貼り付け]                     │   │
│  │                                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│  [📋 Paste & Apply All]                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### AP設定のNVS保存

```cpp
// NVSキー
ap_ssid      // APモードSSID（デフォルト: "Aranea-{type}-{mac6}"）
ap_pass      // APモードパスワード（空=オープン、推奨: "aranea123"）
ap_timeout   // APモードタイムアウト（秒、デフォルト: 300 = 5分）
ap_channel   // APチャンネル（デフォルト: 1）

// フラグ
setup_complete  // 初期設定完了フラグ（0=未完了、1=完了）
```

### Captive Portal実装

```cpp
// DNSサーバー起動（全ドメイン→192.168.4.1）
DNSServer dnsServer;
dnsServer.start(53, "*", WiFi.softAPIP());

// リダイレクトハンドラ
server_->onNotFound([this]() {
  server_->sendHeader("Location", "http://192.168.4.1/setup");
  server_->send(302, "text/plain", "Redirect to setup");
});

// 設定画面
server_->on("/setup", HTTP_GET, [this]() { handleSetupPage(); });
server_->on("/setup/wifi", HTTP_POST, [this]() { handleSetupWiFi(); });
server_->on("/setup/speeddial", HTTP_POST, [this]() { handleSetupSpeedDial(); });
```

---

## ESP32固有の制約事項

### メモリ制約

| 項目 | 制限値 | 対策 |
|------|--------|------|
| HTML最大サイズ | ~32KB | PROGMEM使用、チャンク送信 |
| JSON最大サイズ | ~8KB | DynamicJsonDocument使用、yield()挿入 |
| 同時接続数 | 4クライアント | WebServer制限 |
| WebSocket | 非推奨 | SSE（Server-Sent Events）を使用 |
| ファイルアップロード | 1MB以下 | OTA用のみ |

### 処理時間制約

| 処理 | 最大時間 | 対策 |
|------|----------|------|
| HTTPレスポンス | 5秒以内 | WDT対策でyield()挿入 |
| WiFiスキャン | 10秒 | 非同期実行 |
| NVS書き込み | 100ms | 連続書き込み避ける |
| SPIFFS書き込み | 1秒 | 大きなファイルはチャンク |

### プラットフォーム別機能差異

ESP32とOrange Pi（is20s）は役割が異なるため、それぞれ固有の機能を持ちます。
SpeedDialは共通フレームワークですが、デバイス固有セクションはプラットフォームごとに異なります。

| プラットフォーム | 固有機能 | SpeedDial固有セクション |
|-----------------|---------|------------------------|
| **ESP32共通** | WiFi/NTP/Cloud/Tenant | [Device][WiFi][NTP][Cloud][Tenant][System] |
| **is04a** | 接点出力制御 | [Triggers][Channels] |
| **is05a** | リードスイッチ入力 | [Rules][Channels] |
| **is10** | ルーター監視(SSH) | [Inspector][Routers] |
| **is10t** | Tapoカメラ監視(ONVIF) | [Cameras] |
| **is20s (Orange Pi)** | tsharkキャプチャ、脅威検知 | [Capture][Post][Sync][Rooms] |

---

## API一覧

### 共通エンドポイント

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | メインHTML（認証後） |
| GET | `/?cic={cic}` | JSON Status（巡回ポーリング用） |
| GET | `/api/status` | デバイスステータス |
| GET | `/api/config` | 全設定取得 |
| POST | `/api/common/network` | ネットワーク設定保存 |
| POST | `/api/common/ap` | APモード設定保存 |
| POST | `/api/common/cloud` | クラウド設定保存 |
| POST | `/api/common/tenant` | テナント設定保存 |
| POST | `/api/system/settings` | システム設定保存 |
| POST | `/api/system/reboot` | 再起動実行 |
| POST | `/api/system/factory-reset` | ファクトリーリセット |
| GET | `/api/speeddial` | SpeedDial設定取得（INI形式） |
| POST | `/api/speeddial` | SpeedDial設定適用 |

### WiFiマルチ設定エンドポイント

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/wifi/list` | WiFi設定一覧（6スロット） |
| POST | `/api/wifi/add` | 設定追加 |
| POST | `/api/wifi/delete` | 設定削除 |
| POST | `/api/wifi/move` | 順序変更（up/down） |
| POST | `/api/wifi/connect` | 即時接続 |
| POST | `/api/wifi/scan` | スキャン開始 |
| GET | `/api/wifi/scan/result` | スキャン結果 |
| POST | `/api/wifi/reset` | デフォルトに戻す |
| POST | `/api/wifi/auto` | 自動接続（リスト順） |

### APモード専用エンドポイント

| Method | Path | Description |
|--------|------|-------------|
| GET | `/setup` | 初期設定画面 |
| POST | `/setup/wifi` | WiFi設定＆接続試行 |
| POST | `/setup/tenant` | テナント設定 |
| POST | `/setup/speeddial` | SpeedDial一括適用 |
| POST | `/setup/complete` | 設定完了＆再起動 |

---

## CSS/JSの共通化

### 共通CSSクラス

```css
/* カラースキーム */
:root {
  --primary: #4c4948;        /* Aranea Gray */
  --primary-light: #6b6a69;
  --accent: #3182ce;         /* Blue */
  --accent-hover: #2b6cb0;
  --bg: #f7fafc;
  --card: #fff;
  --border: #e2e8f0;
  --text: #2d3748;
  --text-muted: #718096;
  --success: #38a169;        /* Green */
  --danger: #e53e3e;         /* Red */
  --warning: #d69e2e;        /* Yellow */
}

/* 主要コンポーネント */
.header          /* ヘッダー（ロゴ＋タイトル） */
.device-banner   /* デバイスバナー（型番＋名前） */
.container       /* メインコンテナ */
.tabs            /* タブナビゲーション */
.tab             /* 個別タブ */
.tab-content     /* タブコンテンツ */
.card            /* カード */
.card-title      /* カードタイトル */
.status-grid     /* ステータスグリッド */
.status-item     /* ステータス項目 */
.form-group      /* フォームグループ */
.form-row        /* フォーム行（横並び） */
.btn             /* ボタン */
.btn-primary     /* プライマリボタン */
.btn-danger      /* 危険ボタン */
.btn-sm          /* 小ボタン */
.btn-group       /* ボタングループ */
.toast           /* トースト通知 */
```

### 共通JavaScript関数

```javascript
// 設定ロード
async function load() { ... }

// レンダリング
function render() { ... }

// タブ切り替え
function showTab(name) { ... }

// ステータス更新（5秒間隔）
async function refreshStatus() { ... }

// POST送信
async function post(url, data) { ... }

// トースト表示
function toast(message) { ... }

// WiFi操作
async function addWifiConfig() { ... }
async function deleteWifiConfig(index) { ... }
async function moveWifiConfig(index, direction) { ... }
async function connectWifi(index) { ... }
async function scanWifi() { ... }
async function autoConnectWifi() { ... }

// SpeedDial操作
async function refreshSpeedDial() { ... }
function copySpeedDial() { ... }
async function applySpeedDial() { ... }
```

---

## 実装チェックリスト

### Phase 1: 基本機能（必須）

- [ ] Network Tab: WiFi 6スロット管理（CRUD）
- [ ] Network Tab: WiFi自動接続ロジック
- [ ] Network Tab: WiFiスキャン機能
- [ ] System Tab: deviceName変更→即時deviceStateReport
- [ ] SpeedDial Tab: INI形式出力
- [ ] SpeedDial Tab: INIパース＆適用

### Phase 2: APモード初期設定

- [ ] APモード自動起動条件
- [ ] Captive Portal（DNSリダイレクト）
- [ ] 初期設定ウィザードUI
- [ ] SpeedDial一括設定（APモード）
- [ ] 設定完了→STAモード移行

### Phase 3: 統合テスト

- [ ] is04a での動作確認
- [ ] is05a での動作確認
- [ ] is10 での動作確認
- [ ] is10t での動作確認
- [ ] メモリ使用量確認（Heap監視）
- [ ] 長時間稼働テスト

---

## 付録: デバイスバナー表示例

```
┌─────────────────────────────────────────────────────────────┐
│ 🕸️ Aranea Device Manager                    is10 v1.2.0    │
├─────────────────────────────────────────────────────────────┤
│ ar-is10 Router Inspector                                    │
│ 本社1F-ルーター監視                                         │
│ ルーターの状態監視とクライアント情報収集                    │
└─────────────────────────────────────────────────────────────┘
```

**デバイス種別による表示**:

| DeviceType | ModelName | ContextDesc |
|------------|-----------|-------------|
| ar-is04a | Relay Controller | 接点出力制御デバイス |
| ar-is05a | Reed Switch Monitor | 8chリードスイッチ入力監視 |
| ar-is06a | Environmental Monitor | 温湿度・気圧監視デバイス |
| ar-is10 | Router Inspector | ルーターの状態監視とクライアント情報収集 |
| ar-is10t | Tapo Camera Monitor | Tapoカメラの状態監視 |
