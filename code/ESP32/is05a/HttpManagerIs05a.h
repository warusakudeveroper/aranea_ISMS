/**
 * HttpManagerIs05a.h
 *
 * is05a専用 Web UI実装
 *
 * AraneaWebUIを継承し、8chチャンネル管理とWebhook設定UIを追加。
 */

#ifndef HTTP_MANAGER_IS05A_H
#define HTTP_MANAGER_IS05A_H

#include "AraneaWebUI.h"

class ChannelManager;
class WebhookManager;

class HttpManagerIs05a : public AraneaWebUI {
public:
    HttpManagerIs05a();

    /**
     * 初期化（拡張版）
     */
    void begin(SettingManager* settings, ChannelManager* channels,
               WebhookManager* webhooks, int port = 80);

    /**
     * チャンネルステータス更新（OLED用）
     */
    void setChannelStatus(int lastChanged, const String& lastChangeTime);

    /**
     * 送信ステータス更新
     */
    void setSendStatus(int successCount, int failCount, const String& lastResult);

protected:
    // ========================================
    // AraneaWebUI オーバーライド
    // ========================================
    void getTypeSpecificStatus(JsonObject& obj) override;
    String generateTypeSpecificTabs() override;
    String generateTypeSpecificTabContents() override;
    String generateTypeSpecificJS() override;
    void registerTypeSpecificEndpoints() override;
    void getTypeSpecificConfig(JsonObject& obj) override;

private:
    ChannelManager* channels_;
    WebhookManager* webhooks_;

    // ステータス
    int lastChangedChannel_;
    String lastChangeTime_;
    int sendSuccessCount_;
    int sendFailCount_;
    String lastSendResult_;

    // APIハンドラ
    void handleChannels();
    void handleChannel();
    void handleChannelConfig();
    void handlePulse();
    void handleWebhookConfig();
    void handleWebhookTest();
};

#endif // HTTP_MANAGER_IS05A_H
