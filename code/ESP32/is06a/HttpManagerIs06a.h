/**
 * HttpManagerIs06a.h
 *
 * IS06A用 Web UI実装
 * AraneaWebUIを継承し、is06a固有のAPI/UIを実装
 *
 * 追加エンドポイント:
 * - POST /api/trigger: トリガー実行（パルス）
 * - POST /api/pwm: PWMデューティ設定
 * - GET/POST /api/trigger/config: トリガー設定
 */

#ifndef HTTP_MANAGER_IS06A_H
#define HTTP_MANAGER_IS06A_H

#include "AraneaWebUI.h"

class TriggerManagerIs06a;

class HttpManagerIs06a : public AraneaWebUI {
public:
    HttpManagerIs06a();

    void begin(SettingManager* settings, TriggerManagerIs06a* trigger, int port);

    // ステータス設定
    void setSendStatus(int sent, int fail, const String& lastResult);
    void setTriggerStatus(int trgNum, const String& lastUpdatedAt);

    // サーバー取得
    WebServer* getServer() { return server_; }

protected:
    // AraneaWebUI オーバーライド
    void registerTypeSpecificEndpoints() override;
    void getTypeSpecificStatus(JsonObject& obj) override;
    void getTypeSpecificConfig(JsonObject& obj) override;
    String generateTypeSpecificTabs() override;
    String generateTypeSpecificJS() override;
    String generateTypeSpecificTabContents() override;

private:
    TriggerManagerIs06a* trigger_;

    int sentCount_;
    int failCount_;
    String lastResult_;
    String lastTriggerUpdatedAt_;
    int lastTriggerNum_;

    // API ハンドラ
    void handleTrigger();       // POST /api/trigger
    void handlePwm();           // POST /api/pwm
    void handleTriggerConfig(); // GET/POST /api/trigger/config
};

#endif // HTTP_MANAGER_IS06A_H
