#pragma once
#include <Arduino.h>

/**
 * AraneaRegister
 *
 * araneaDeviceGate APIを使用してデバイスをmobes2.0に登録
 * 登録成功時にCICを取得し、NVSに保存
 */

struct AraneaRegisterResult {
  bool ok = false;
  String cic_code;       // 6桁CIC
  String stateEndpoint;  // deviceStateReport URL
  String mqttEndpoint;   // MQTT WebSocket URL (双方向デバイスのみ)
  String error;
};

struct TenantPrimaryAuth {
  String lacisId;
  String userId;
  String cic;
  // passは廃止（認証はlacisId + userId + cicの3要素）
};

class AraneaRegister {
public:
  /**
   * 初期化
   * @param gateUrl araneaDeviceGate エンドポイントURL
   */
  void begin(const String& gateUrl);

  /**
   * テナントPrimary認証情報を設定
   */
  void setTenantPrimary(const TenantPrimaryAuth& auth);

  /**
   * デバイス登録
   * @param tid テナントID
   * @param deviceType デバイスタイプ (例: "ISMS_ar-is02")
   * @param lacisId デバイスのlacisId
   * @param macAddress MACアドレス (12桁HEX)
   * @param productType プロダクトタイプ (3桁)
   * @param productCode プロダクトコード (4桁)
   * @return 登録結果
   */
  AraneaRegisterResult registerDevice(
    const String& tid,
    const String& deviceType,
    const String& lacisId,
    const String& macAddress,
    const String& productType,
    const String& productCode
  );

  /**
   * 登録済みかチェック（NVSにCICが保存されているか）
   */
  bool isRegistered();

  /**
   * 保存されたCICを取得
   */
  String getSavedCic();

  /**
   * 保存されたstateEndpointを取得
   */
  String getSavedStateEndpoint();

  /**
   * 保存されたmqttEndpointを取得（双方向デバイスのみ）
   */
  String getSavedMqttEndpoint();

  /**
   * 登録データをクリア（再登録を強制）
   * NVSから cic, stateEndpoint, mqttEndpoint, registered フラグを削除
   */
  void clearRegistration();

private:
  String gateUrl_;
  TenantPrimaryAuth tenantAuth_;

  // NVSキー
  static const char* NVS_NAMESPACE;
  static const char* KEY_CIC;
  static const char* KEY_STATE_ENDPOINT;
  static const char* KEY_MQTT_ENDPOINT;
  static const char* KEY_REGISTERED;
};
