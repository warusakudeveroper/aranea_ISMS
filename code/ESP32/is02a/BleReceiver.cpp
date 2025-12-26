/**
 * BleReceiver.cpp
 *
 * IS02A用 BLE受信マネージャー
 */

#include "BleReceiver.h"
#include "LacisIDGenerator.h"

void BleReceiver::begin(int scanDurationSec, const String& filterPrefix) {
  scanDurationSec_ = scanDurationSec;
  filterPrefix_ = filterPrefix;

  // NimBLE初期化
  NimBLEDevice::init("is02a");

  // スキャン設定
  pBLEScan_ = NimBLEDevice::getScan();
  pBLEScan_->setScanCallbacks(this, false);
  pBLEScan_->setActiveScan(true);
  pBLEScan_->setInterval(100);
  pBLEScan_->setWindow(99);

  Serial.println("[BleReceiver] Initialized");
}

void BleReceiver::startContinuousScan() {
  if (pBLEScan_ == nullptr) return;

  // 継続スキャン開始 (duration=0, isContinue=true)
  pBLEScan_->start(0, true);
  scanning_ = true;
  Serial.println("[BleReceiver] Continuous scan started");
}

void BleReceiver::stopScan() {
  if (pBLEScan_ == nullptr) return;

  pBLEScan_->stop();
  scanning_ = false;
  Serial.println("[BleReceiver] Scan stopped");
}

void BleReceiver::cleanupExpired() {
  unsigned long now = millis();

  portENTER_CRITICAL(&nodesMux_);
  for (auto& node : nodes_) {
    if (node.valid && (now - node.receivedAt > BLE_NODE_EXPIRE_MS)) {
      node.valid = false;
      Serial.printf("[BleReceiver] Node expired: %s\n", node.lacisId.c_str());
    }
  }
  portEXIT_CRITICAL(&nodesMux_);
}

std::vector<BleNodeData> BleReceiver::getNodes() const {
  // コピーを返す（スレッドセーフ）
  portENTER_CRITICAL((portMUX_TYPE*)&nodesMux_);
  std::vector<BleNodeData> copy = nodes_;
  portEXIT_CRITICAL((portMUX_TYPE*)&nodesMux_);
  return copy;
}

int BleReceiver::getValidNodeCount() const {
  int count = 0;
  portENTER_CRITICAL((portMUX_TYPE*)&nodesMux_);
  for (const auto& node : nodes_) {
    if (node.valid) count++;
  }
  portEXIT_CRITICAL((portMUX_TYPE*)&nodesMux_);
  return count;
}

void BleReceiver::clearNodes() {
  portENTER_CRITICAL(&nodesMux_);
  nodes_.clear();
  portEXIT_CRITICAL(&nodesMux_);
  Serial.println("[BleReceiver] Nodes cleared");
}

void BleReceiver::onResult(const NimBLEAdvertisedDevice* advertisedDevice) {
  // Manufacturer Data 取得
  if (!advertisedDevice->haveManufacturerData()) return;

  std::string mfgData = advertisedDevice->getManufacturerData();

  // ISMSv1 判定
  if (!BleIsmsParser::isIsmsV1(mfgData)) return;

  // パース
  IsmsPayload payload;
  if (!BleIsmsParser::parse(mfgData, &payload)) return;

  // フィルターチェック
  if (filterPrefix_.length() > 0) {
    String lacisId = generateLacisId(payload.productType, payload.staMac12Hex);
    if (!lacisId.startsWith(filterPrefix_)) return;
  }

  // ノード追加/更新
  upsertNode(payload, advertisedDevice->getRSSI());
}

void BleReceiver::upsertNode(const IsmsPayload& payload, int rssi) {
  String lacisId = generateLacisId(payload.productType, payload.staMac12Hex);

  portENTER_CRITICAL(&nodesMux_);

  // 既存ノード検索
  bool found = false;
  for (auto& node : nodes_) {
    if (node.lacisId == lacisId) {
      // 更新
      node.temperature = payload.tempC;
      node.humidity = payload.humPct;
      node.battery = payload.battPct;
      node.rssi = rssi;
      node.receivedAt = millis();
      node.bootCount = payload.bootCount;
      node.valid = true;
      found = true;
      break;
    }
  }

  // 新規追加
  if (!found && nodes_.size() < BLE_MAX_NODES) {
    BleNodeData node;
    node.lacisId = lacisId;
    node.mac = payload.staMac12Hex;
    node.productType = String(payload.productType);
    // 3桁にパディング
    while (node.productType.length() < 3) {
      node.productType = "0" + node.productType;
    }
    node.temperature = payload.tempC;
    node.humidity = payload.humPct;
    node.battery = payload.battPct;
    node.rssi = rssi;
    node.receivedAt = millis();
    node.bootCount = payload.bootCount;
    node.valid = true;
    nodes_.push_back(node);

    Serial.printf("[BleReceiver] New node: %s (%.1fC, %.1f%%, %d%%)\n",
      lacisId.c_str(), payload.tempC, payload.humPct, payload.battPct);
  }

  portEXIT_CRITICAL(&nodesMux_);
}

String BleReceiver::generateLacisId(uint8_t productType, const String& staMac12) {
  // LacisID形式: [Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁]
  // is01aのProductCode: 0001
  char buf[24];
  snprintf(buf, sizeof(buf), "3%03d%s0001", productType, staMac12.c_str());
  return String(buf);
}
