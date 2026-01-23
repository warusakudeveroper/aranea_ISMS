/**
 * HttpManagerIs06s.h
 *
 * IS06S HTTP API & Web UI マネージャー
 *
 * AraneaWebUIを継承し、PIN制御機能を追加
 *
 * 【APIエンドポイント】
 * - GET /api/status          - 全体ステータス（typeSpecific含む）
 * - GET /api/pin/{ch}/state  - PIN状態取得
 * - POST /api/pin/{ch}/state - PIN状態設定
 * - GET /api/pin/{ch}/setting - PIN設定取得
 * - POST /api/pin/{ch}/setting - PIN設定変更
 * - GET /api/settings        - 全設定取得
 * - POST /api/settings       - 全設定保存
 *
 * 【Web UIタブ】
 * - PINControl: リアルタイムPIN操作
 * - PINSettings: PIN設定変更
 */

#ifndef HTTP_MANAGER_IS06S_H
#define HTTP_MANAGER_IS06S_H

#include <Arduino.h>
#include "AraneaWebUI.h"
#include "Is06PinManager.h"

class HttpManagerIs06s : public AraneaWebUI {
public:
  HttpManagerIs06s();
  virtual ~HttpManagerIs06s();

  /**
   * 初期化
   * @param settings SettingManager参照
   * @param pinManager Is06PinManager参照
   * @param port HTTPポート（デフォルト80）
   */
  void begin(SettingManager* settings, Is06PinManager* pinManager, int port = 80);

protected:
  // ========================================
  // AraneaWebUI オーバーライド
  // ========================================

  /**
   * typeSpecificステータス追加（PIN状態）
   */
  void getTypeSpecificStatus(JsonObject& obj) override;

  /**
   * 固有タブ（PINControl, PINSettings）
   */
  String generateTypeSpecificTabs() override;

  /**
   * 固有タブコンテンツ
   */
  String generateTypeSpecificTabContents() override;

  /**
   * 固有JavaScript
   */
  String generateTypeSpecificJS() override;

  /**
   * 固有APIエンドポイント登録
   */
  void registerTypeSpecificEndpoints() override;

  /**
   * 固有設定取得
   */
  void getTypeSpecificConfig(JsonObject& obj) override;

private:
  Is06PinManager* pinManager_ = nullptr;

  // ========================================
  // APIハンドラ
  // ========================================
  void handlePinStateGet();
  void handlePinStatePost();
  void handlePinSettingGet();
  void handlePinSettingPost();
  void handlePinToggle();
  void handlePinAll();
  void handleSettingsGet();
  void handleSettingsPost();

  // ========================================
  // ヘルパー
  // ========================================
  int extractChannelFromUri();
  void sendJsonResponse(int code, const String& message);
  void sendJsonSuccess(const String& message = "OK");
  void sendJsonError(int code, const String& message);

  // JSON構築
  void buildPinStateJson(JsonObject& obj, int channel);
  void buildPinSettingJson(JsonObject& obj, int channel);
  void buildAllPinsJson(JsonArray& arr);
};

#endif // HTTP_MANAGER_IS06S_H
