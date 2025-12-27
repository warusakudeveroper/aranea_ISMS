/**
 * TapoPollerIs10t.h
 *
 * Tapoカメラ巡回ポーラー
 *
 * 責務:
 * - 複数カメラの順次ポーリング
 * - ONVIF経由でのデバイス情報取得
 * - オンライン/オフライン状態管理
 */

#ifndef TAPO_POLLER_IS10T_H
#define TAPO_POLLER_IS10T_H

#include <Arduino.h>
#include "TapoTypes.h"
#include "OnvifClient.h"
#include "TapoDiscovery.h"

// 前方宣言
class SettingManager;

// ポーリング間隔（5分）
#define TAPO_POLL_INTERVAL_MS 300000

class TapoPollerIs10t {
public:
    TapoPollerIs10t();

    /**
     * 初期化
     */
    void begin(SettingManager* settings);

    /**
     * メインループ処理
     * @return ポーリング完了時true
     */
    bool handle();

    /**
     * 手動ポーリング開始
     */
    void startPoll();

    /**
     * ポーリング中かどうか
     */
    bool isPolling() const { return polling_; }

    /**
     * 現在ポーリング中のカメラインデックス
     */
    uint8_t getCurrentIndex() const { return currentIndex_; }

    /**
     * カメラ数取得
     */
    uint8_t getCameraCount() const { return cameraCount_; }

    /**
     * カメラ設定取得
     */
    const TapoConfig& getConfig(uint8_t index) const;

    /**
     * カメラステータス取得
     */
    const TapoStatus& getStatus(uint8_t index) const;

    /**
     * サマリ取得
     */
    const TapoPollSummary& getSummary() const { return summary_; }

    /**
     * カメラ追加
     */
    bool addCamera(const TapoConfig& config);

    /**
     * カメラ更新
     */
    bool updateCamera(uint8_t index, const TapoConfig& config);

    /**
     * カメラ削除
     */
    bool removeCamera(uint8_t index);

    /**
     * 設定保存
     */
    void saveConfig();

    /**
     * 設定読み込み
     */
    void loadConfig();

private:
    SettingManager* settings_;

    TapoConfig configs_[MAX_TAPO_CAMERAS];
    TapoStatus statuses_[MAX_TAPO_CAMERAS];
    uint8_t cameraCount_;

    TapoPollSummary summary_;

    bool polling_;
    uint8_t currentIndex_;
    unsigned long lastPollMs_;
    unsigned long lastCameraMs_;

    OnvifClient onvifClient_;
    TapoDiscovery discovery_;

    /**
     * 単一カメラをポーリング
     */
    bool pollCamera(uint8_t index);

    /**
     * サマリ更新
     */
    void updateSummary();
};

#endif // TAPO_POLLER_IS10T_H
