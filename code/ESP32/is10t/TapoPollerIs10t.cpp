/**
 * TapoPollerIs10t.cpp
 *
 * Tapoカメラ巡回ポーラー実装
 */

#include "TapoPollerIs10t.h"
#include "SettingManager.h"

// NVSキー
static const char* KEY_TAPO_COUNT = "tapo_count";
static const char* KEY_TAPO_HOST = "tapo_%d_host";
static const char* KEY_TAPO_PORT = "tapo_%d_port";
static const char* KEY_TAPO_USER = "tapo_%d_user";
static const char* KEY_TAPO_PASS = "tapo_%d_pass";
static const char* KEY_TAPO_ENABLED = "tapo_%d_en";

TapoPollerIs10t::TapoPollerIs10t() {
    settings_ = nullptr;
    cameraCount_ = 0;
    polling_ = false;
    currentIndex_ = 0;
    lastPollMs_ = 0;
    lastCameraMs_ = 0;
}

void TapoPollerIs10t::begin(SettingManager* settings) {
    settings_ = settings;
    loadConfig();
    Serial.printf("[TapoPoller] Initialized with %d cameras\n", cameraCount_);
}

void TapoPollerIs10t::loadConfig() {
    if (!settings_) return;

    cameraCount_ = settings_->getInt(KEY_TAPO_COUNT, 0);
    if (cameraCount_ > MAX_TAPO_CAMERAS) {
        cameraCount_ = MAX_TAPO_CAMERAS;
    }

    char keyBuf[20];
    for (uint8_t i = 0; i < cameraCount_; i++) {
        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_HOST, i);
        String host = settings_->getString(keyBuf, "");
        strncpy(configs_[i].host, host.c_str(), sizeof(configs_[i].host) - 1);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_PORT, i);
        configs_[i].onvifPort = settings_->getInt(keyBuf, TAPO_ONVIF_PORT);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_USER, i);
        String user = settings_->getString(keyBuf, "");
        strncpy(configs_[i].username, user.c_str(), sizeof(configs_[i].username) - 1);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_PASS, i);
        String pass = settings_->getString(keyBuf, "");
        strncpy(configs_[i].password, pass.c_str(), sizeof(configs_[i].password) - 1);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_ENABLED, i);
        configs_[i].enabled = settings_->getBool(keyBuf, true);

        Serial.printf("[TapoPoller] Camera %d: %s:%d (%s)\n",
                      i, configs_[i].host, configs_[i].onvifPort,
                      configs_[i].enabled ? "enabled" : "disabled");
    }
}

void TapoPollerIs10t::saveConfig() {
    if (!settings_) return;

    settings_->setInt(KEY_TAPO_COUNT, cameraCount_);

    char keyBuf[20];
    for (uint8_t i = 0; i < cameraCount_; i++) {
        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_HOST, i);
        settings_->setString(keyBuf, configs_[i].host);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_PORT, i);
        settings_->setInt(keyBuf, configs_[i].onvifPort);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_USER, i);
        settings_->setString(keyBuf, configs_[i].username);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_PASS, i);
        settings_->setString(keyBuf, configs_[i].password);

        snprintf(keyBuf, sizeof(keyBuf), KEY_TAPO_ENABLED, i);
        settings_->setBool(keyBuf, configs_[i].enabled);
    }

    // NVS auto-commits on each set operation
}

bool TapoPollerIs10t::handle() {
    unsigned long now = millis();

    // ポーリング開始判定
    if (!polling_) {
        if (cameraCount_ > 0 && (lastPollMs_ == 0 || now - lastPollMs_ >= TAPO_POLL_INTERVAL_MS)) {
            startPoll();
        }
        return false;
    }

    // カメラ間待機
    if (now - lastCameraMs_ < TAPO_CAMERA_GAP_MS) {
        return false;
    }

    // 次のカメラをポーリング
    while (currentIndex_ < cameraCount_) {
        if (configs_[currentIndex_].enabled) {
            pollCamera(currentIndex_);
            currentIndex_++;
            lastCameraMs_ = now;
            return false;
        }
        currentIndex_++;
    }

    // 全カメラ完了
    polling_ = false;
    lastPollMs_ = now;
    updateSummary();

    Serial.printf("[TapoPoller] Poll complete: %d online, %d offline\n",
                  summary_.online, summary_.offline);

    return true;
}

void TapoPollerIs10t::startPoll() {
    if (cameraCount_ == 0) return;

    polling_ = true;
    currentIndex_ = 0;
    lastCameraMs_ = 0;

    Serial.println("[TapoPoller] Starting poll cycle");
}

bool TapoPollerIs10t::pollCamera(uint8_t index) {
    if (index >= cameraCount_) return false;

    TapoConfig& config = configs_[index];
    TapoStatus& status = statuses_[index];

    Serial.printf("[TapoPoller] Polling camera %d: %s\n", index, config.host);

    // Issue fix: Preserve failCount and online state before clearing
    uint8_t savedFailCount = status.failCount;
    bool savedOnline = status.online;

    // Clear status fields to avoid stale data
    status = TapoStatus();

    // Restore preserved state
    status.failCount = savedFailCount;
    // online will be determined by poll result below

    // ONVIF接続設定
    onvifClient_.setTarget(config.host, config.onvifPort);
    onvifClient_.setCredentials(config.username, config.password);

    // ONVIF情報取得
    bool onvifSuccess = onvifClient_.getAllInfo(&status);

    if (onvifSuccess) {
        status.online = true;
        status.failCount = 0;
        status.lastPollMs = millis();

        Serial.printf("[TapoPoller] Camera %d ONVIF OK: %s (%s)\n",
                      index, status.model, status.getMacString().c_str());

        // Discovery情報取得（ONVIF成功時のみ）
        // Note: discoveryがタイムアウトしてもONVIF成功なら online=true
        if (discovery_.discover(config.host, &status)) {
            Serial.printf("[TapoPoller] Camera %d Discovery OK: %s\n",
                          index, status.deviceName);
        } else {
            Serial.printf("[TapoPoller] Camera %d Discovery FAIL: %s\n",
                          index, discovery_.getLastError().c_str());
        }
    } else {
        status.failCount++;

        // Issue fix: Only mark offline when threshold reached
        // Keep previous online state if under threshold
        if (status.failCount >= TAPO_FAIL_THRESHOLD) {
            status.online = false;
            Serial.printf("[TapoPoller] Camera %d OFFLINE (threshold reached)\n", index);
        } else {
            status.online = savedOnline;  // Maintain previous state
            Serial.printf("[TapoPoller] Camera %d FAIL (%d/%d), keeping online=%s: %s\n",
                          index, status.failCount, TAPO_FAIL_THRESHOLD,
                          savedOnline ? "true" : "false",
                          onvifClient_.getLastError().c_str());
        }
    }

    return onvifSuccess;
}

void TapoPollerIs10t::updateSummary() {
    summary_.total = 0;
    summary_.online = 0;
    summary_.offline = 0;

    for (uint8_t i = 0; i < cameraCount_; i++) {
        if (configs_[i].enabled) {
            summary_.total++;
            if (statuses_[i].online) {
                summary_.online++;
            } else {
                summary_.offline++;
            }
        }
    }

    summary_.lastPollMs = millis();
}

const TapoConfig& TapoPollerIs10t::getConfig(uint8_t index) const {
    static TapoConfig empty;
    if (index >= cameraCount_) return empty;
    return configs_[index];
}

const TapoStatus& TapoPollerIs10t::getStatus(uint8_t index) const {
    static TapoStatus empty;
    if (index >= cameraCount_) return empty;
    return statuses_[index];
}

bool TapoPollerIs10t::addCamera(const TapoConfig& config) {
    if (cameraCount_ >= MAX_TAPO_CAMERAS) return false;

    configs_[cameraCount_] = config;
    statuses_[cameraCount_] = TapoStatus();
    cameraCount_++;

    saveConfig();
    return true;
}

bool TapoPollerIs10t::updateCamera(uint8_t index, const TapoConfig& config) {
    if (index >= cameraCount_) return false;

    configs_[index] = config;
    saveConfig();
    return true;
}

bool TapoPollerIs10t::removeCamera(uint8_t index) {
    if (index >= cameraCount_) return false;

    // シフト
    for (uint8_t i = index; i < cameraCount_ - 1; i++) {
        configs_[i] = configs_[i + 1];
        statuses_[i] = statuses_[i + 1];
    }

    cameraCount_--;
    saveConfig();
    return true;
}
