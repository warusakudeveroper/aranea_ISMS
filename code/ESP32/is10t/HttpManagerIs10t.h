/**
 * HttpManagerIs10t.h
 *
 * IS10T (Tapo Monitor) HTTPサーバー
 * AraneaWebUI 基底クラスを継承し、is10t固有の機能を追加
 */

#ifndef HTTP_MANAGER_IS10T_H
#define HTTP_MANAGER_IS10T_H

#include <Arduino.h>
#include "AraneaWebUI.h"
#include "TapoTypes.h"

// 前方宣言
class TapoPollerIs10t;

class HttpManagerIs10t : public AraneaWebUI {
public:
    /**
     * 初期化
     * @param settings SettingManagerインスタンス
     * @param poller TapoPollerインスタンス
     * @param port HTTPポート
     */
    void begin(SettingManager* settings, TapoPollerIs10t* poller, int port = 80);

    /**
     * ポーリングステータス更新
     */
    void setPollingStatus(bool polling, uint8_t currentCamera);

protected:
    // AraneaWebUI オーバーライド
    void getTypeSpecificStatus(JsonObject& obj) override;
    void getTypeSpecificConfig(JsonObject& obj) override;
    String generateTypeSpecificTabs() override;
    String generateTypeSpecificTabContents() override;
    String generateTypeSpecificJS() override;
    void registerTypeSpecificEndpoints() override;

private:
    TapoPollerIs10t* poller_ = nullptr;

    bool pollingInProgress_ = false;
    uint8_t currentCameraIndex_ = 0;

    // is10t固有ハンドラ
    void handleGetCameras();
    void handleAddCamera();
    void handleUpdateCamera();
    void handleDeleteCamera();
    void handleStartPoll();
};

#endif // HTTP_MANAGER_IS10T_H
