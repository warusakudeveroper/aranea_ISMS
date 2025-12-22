/**
 * HttpManagerIs04a.h
 *
 * IS04A用 Web UI実装
 * AraneaWebUIを継承し、is04a固有のAPI/UIを実装
 */

#ifndef HTTP_MANAGER_IS04A_H
#define HTTP_MANAGER_IS04A_H

#include "AraneaWebUI.h"

class TriggerManager;

class HttpManagerIs04a : public AraneaWebUI {
public:
    HttpManagerIs04a();

    void begin(SettingManager* settings, TriggerManager* trigger, int port);

    // ステータス設定
    void setSendStatus(int sent, int fail, const String& lastResult);
    void setPulseStatus(int outputNum, const String& lastUpdatedAt);

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
    TriggerManager* trigger_;

    int sentCount_;
    int failCount_;
    String lastResult_;
    String lastPulseUpdatedAt_;
    int lastPulseOutput_;

    // API ハンドラ
    void handlePulse();
    void handlePulseConfig();
    void handleTriggerConfig();
};

#endif // HTTP_MANAGER_IS04A_H
