# is05a 8chディテクター実装報告

**作成日**: 2025-12-27
**ステータス**: 本番デプロイ完了

## デバイス概要

| 項目 | 値 |
|------|-----|
| 型番 | is05a (aranea_ar-is05a) |
| 機能 | 8ch接点入力（ch1-6入力、ch7-8出力対応） |
| テナント | T2025120621041161827 |
| テナント管理者 | soejim@mijeos.com |
| CIC | 204965 |

## 修正した問題

### 1. MQTT状態報告が常にfalse

**症状**: HTTP API `/api/status` で `mqttConnected: false` と表示される（実際は接続中）

**原因**: HttpManagerIs05aに `setMqttConnected()` メソッドがなかった。AraneaWebUI基底クラスの `getCloudStatus()` がデフォルトでfalseを返す。

**修正内容**:

```cpp
// HttpManagerIs05a.h に追加
void setMqttConnected(bool connected) { mqttConnected_ = connected; }
bool isMqttConnected() const { return mqttConnected_; }

protected:
    AraneaCloudStatus getCloudStatus() override;

private:
    bool mqttConnected_ = false;

// HttpManagerIs05a.cpp に追加
AraneaCloudStatus HttpManagerIs05a::getCloudStatus() {
    AraneaCloudStatus status;
    status.registered = deviceInfo_.cic.length() > 0;
    status.mqttConnected = mqttConnected_;
    status.lastStateReport = "";
    status.lastStateReportCode = 0;
    status.schemaVersion = 0;
    return status;
}

// is05a.ino のMQTTコールバックで呼び出し
mqtt.onConnected([]() {
    httpMgr.setMqttConnected(true);
    // ...
});
mqtt.onDisconnected([]() {
    httpMgr.setMqttConnected(false);
});
```

### 2. 状態文字列の不整合

**症状**: 入力チャンネルが "open/close" 表記で実態とマッチしない

**修正**: ChannelManager.cpp `getStateString()`
- 入力チャンネル: "detect" / "idle"
- 出力チャンネル: "on" / "off"

```cpp
String ChannelManager::getStateString(int ch) const {
    if (ch < 1 || ch > NUM_CHANNELS) return "";
    const ChannelConfig& cfg = configs_[ch - 1];
    bool active = channels_[ch - 1].isActive();

    // 出力チャンネル（ch7/ch8）は on/off
    if (cfg.isOutput) {
        return active ? "on" : "off";
    }
    // 入力チャンネルは detect/idle
    return active ? "detect" : "idle";
}
```

## 発見した問題（未解決）

### シリアル接続時の不安定化

**症状**: シリアルモニター接続中にデバイスが不安定になる（リブートループ的挙動）

**推測原因**:
1. DTR/RTS信号によるハードウェアリセット
2. シリアル出力のバッファオーバーフロー
3. ログ出力過多による処理遅延
4. モニターツールのbaud rate不一致

**暫定対策**: 本番運用ではシリアル接続を避ける。デバッグ時は短時間のみ使用。

## 本番設定

### AraneaSettings.h

```cpp
// テナント設定
#define ARANEA_DEFAULT_TID "T2025120621041161827"
#define ARANEA_DEFAULT_FID "0099"
#define ARANEA_DEFAULT_TENANT_LACISID "18217487937895888001"
#define ARANEA_DEFAULT_TENANT_EMAIL "soejim@mijeos.com"
#define ARANEA_DEFAULT_TENANT_CIC "204965"

// WiFi設定
#define ARANEA_DEFAULT_WIFI_SSID_1 "H_to_facility"
#define ARANEA_DEFAULT_WIFI_PASS_1 "a,9E%JJDQ&kj"
#define ARANEA_DEFAULT_WIFI_SSID_6 "H_to_facility"
#define ARANEA_DEFAULT_WIFI_PASS_6 "a,9E%JJDQ&kj"

// MQTT設定
#define ARANEA_DEFAULT_MQTT_BROKER "wss://aranea-mqtt-bridge-1010551946141.asia-northeast1.run.app"
#define ARANEA_DEFAULT_MQTT_PORT 443
```

## フラッシュ済みデバイス一覧

| # | MAC | lacisID | 備考 |
|---|-----|---------|------|
| 1 | 6C:C8:40:8C:31:B0 | 30066CC8408C31B00001 | |
| 2 | 6C:C8:40:8C:A3:E0 | 30066CC8408CA3E00001 | |
| 3 | 6C:C8:40:8C:F3:1C | 30066CC8408CF31C0001 | |
| 4 | 6C:C8:40:8D:02:44 | 30066CC8408D02440001 | |
| 5 | (未フラッシュ) | - | 要対応 |

## MQTTコマンド対応

| コマンド | パラメータ | 動作 |
|----------|-----------|------|
| `get_status` | なし | 全チャンネル状態取得 |
| `output_on` | `channel: 7-8` | 出力ON |
| `output_off` | `channel: 7-8` | 出力OFF |
| `set_config` | 各種設定 | 設定変更 |

### コマンド例（SDK経由）

```bash
# ステータス取得
$ARANEA mqtt test -l <lacisID> --tid <TID> -a status -e production

# 出力ON
$ARANEA mqtt send-command -l <lacisID> --tid <TID> \
  --type output_on --params '{"channel":7}' -e production
```

## ビルド情報

- **パーティション**: min_spiffs（OTA対応維持）
- **フラッシュサイズ**: 1,493,680 bytes (75%)
- **RAM使用**: 55,496 bytes (16%)

## インストール手順

```bash
# 1. フラッシュ消去（NVSクリア必須）
esptool --chip esp32 --port /dev/cu.usbserial-0001 erase_flash

# 2. ファームウェア書き込み
esptool --chip esp32 --port /dev/cu.usbserial-0001 --baud 921600 write_flash \
  0x1000 is05a.ino.bootloader.bin \
  0x8000 is05a.ino.partitions.bin \
  0x10000 is05a.ino.bin
```

## 教訓

1. **ビルドキャッシュに注意**: 設定変更後は必ず再ビルド確認。`/tmp` のバイナリが古い可能性
2. **クリーンインストール必須**: NVSに旧設定が残るため `erase_flash` 実行
3. **シリアル接続は開発時のみ**: 本番運用では接続しない
4. **is02aを参考にする**: MQTT状態報告など、動作実績のあるis02aのコードを確認

## 関連ファイル

- `code/ESP32/is05a/` - メインコード
- `code/ESP32/is05a/AraneaSettings.h` - 本番設定
- `code/ESP32/is05a/HttpManagerIs05a.cpp` - HTTP API実装
- `code/ESP32/is05a/ChannelManager.cpp` - チャンネル管理
