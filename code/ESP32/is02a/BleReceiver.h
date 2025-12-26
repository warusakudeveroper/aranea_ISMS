/**
 * BleReceiver.h
 *
 * IS02A用 BLE受信マネージャー
 * is01a(Cluster Node)からのISMSv1アドバタイズを受信・蓄積
 */

#ifndef BLE_RECEIVER_IS02A_H
#define BLE_RECEIVER_IS02A_H

#include <Arduino.h>
#include <vector>
#include <NimBLEDevice.h>
#include "BleIsmsParser.h"

// 受信ノード最大数
#define BLE_MAX_NODES 32

// ノードデータ有効期限（ミリ秒）
#define BLE_NODE_EXPIRE_MS 180000  // 3分

/**
 * 受信ノードデータ
 */
struct BleNodeData {
  String lacisId;           // LacisID (20文字)
  String mac;               // MACアドレス(12桁HEX)
  String productType;       // 製品タイプ(3桁)
  float temperature;        // 温度(℃)
  float humidity;           // 湿度(%)
  uint8_t battery;          // バッテリー(%)
  int rssi;                 // 受信RSSI
  unsigned long receivedAt; // 受信時刻(millis)
  uint32_t bootCount;       // 起動回数
  bool valid;               // 有効フラグ
};

/**
 * BleReceiver - BLE受信マネージャー
 */
class BleReceiver : public NimBLEScanCallbacks {
public:
  /**
   * 初期化
   * @param scanDurationSec スキャン時間（秒）
   * @param filterPrefix LacisIDプレフィックスフィルター（空文字で全受信）
   */
  void begin(int scanDurationSec = 5, const String& filterPrefix = "");

  /**
   * 継続スキャン開始（バックグラウンド）
   */
  void startContinuousScan();

  /**
   * スキャン停止
   */
  void stopScan();

  /**
   * 期限切れノードを削除
   */
  void cleanupExpired();

  /**
   * 受信ノードリスト取得
   */
  std::vector<BleNodeData> getNodes() const;

  /**
   * 有効ノード数取得
   */
  int getValidNodeCount() const;

  /**
   * ノードリストクリア
   */
  void clearNodes();

  /**
   * スキャン時間設定
   */
  void setScanDuration(int sec) { scanDurationSec_ = sec; }

  /**
   * フィルター設定
   */
  void setFilterPrefix(const String& prefix) { filterPrefix_ = prefix; }

  /**
   * スキャン中かどうか
   */
  bool isScanning() const { return scanning_; }

  // NimBLEコールバック
  void onResult(const NimBLEAdvertisedDevice* advertisedDevice) override;

private:
  NimBLEScan* pBLEScan_ = nullptr;
  std::vector<BleNodeData> nodes_;
  int scanDurationSec_ = 5;
  String filterPrefix_ = "";
  bool scanning_ = false;
  portMUX_TYPE nodesMux_ = portMUX_INITIALIZER_UNLOCKED;

  /**
   * ノードを追加または更新
   */
  void upsertNode(const IsmsPayload& payload, int rssi);

  /**
   * LacisID生成（STA MACから）
   */
  String generateLacisId(uint8_t productType, const String& staMac12);
};

#endif // BLE_RECEIVER_IS02A_H
