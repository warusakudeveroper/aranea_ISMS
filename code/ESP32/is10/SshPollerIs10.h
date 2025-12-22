/**
 * SshPollerIs10.h
 *
 * IS10 ルーターSSH巡回ポーラー
 *
 * 機能:
 * - 複数ルーターへの順次SSH接続
 * - FreeRTOSタスクによる非同期実行
 * - ルーター情報収集（WAN IP, LAN IP, SSID, クライアント数等）
 * - ウォッチドッグによるタイムアウト管理
 *
 * 設計改善点（P0対応）:
 * - delay()撤去 → nextAt方式（非ブロッキング）
 * - ランダム開始インデックス（特定ルーターでのハング防止）
 * - タスクにindex固定渡し（競合防止）
 * - watchdog発火時はrestart flag（タスク積み上げ防止）
 */

#ifndef SSH_POLLER_IS10_H
#define SSH_POLLER_IS10_H

#include <Arduino.h>
#include <functional>
#include "RouterTypes.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

// ========================================
// ルーター情報構造体（取得データ）
// ========================================
struct RouterInfo {
  String rid;
  String lacisId;       // ルーターのLacisID（MACから生成）
  String routerMac;     // ルーターのMAC（大文字12HEX）
  String wanIp;
  String lanIp;
  String ssid24;        // 2.4GHz SSID
  String ssid50;        // 5.0GHz SSID
  String ssidMix;       // 混合SSID
  int clientCount;
  String vendorInfo;
  String modelName;
  unsigned long lastUpdate;
  bool online;
};

// ========================================
// SSHタスクパラメータ（index固定渡し用）
// ========================================
struct SshTaskParams {
  class SshPollerIs10* poller;
  int routerIndex;
};

// ========================================
// SshPollerIs10 クラス
// ========================================
class SshPollerIs10 {
public:
  SshPollerIs10();
  ~SshPollerIs10();

  /**
   * 初期化
   * @param routers ルーター設定配列
   * @param routerCount ルーター数へのポインタ
   * @param sshTimeout SSHタイムアウト（ミリ秒）
   */
  void begin(RouterConfig* routers, int* routerCount, unsigned long sshTimeout = 90000);

  /**
   * ループ処理
   * main loop()で毎回呼び出す
   * 非ブロッキング（delay使用なし）
   */
  void handle();

  /**
   * ポーリング有効/無効設定
   */
  void setEnabled(bool enabled) { enabled_ = enabled; }
  bool isEnabled() const { return enabled_; }

  /**
   * ポーリング状態取得
   */
  bool isPolling() const { return sshInProgress_; }
  int getCurrentIndex() const { return currentRouterIndex_; }
  int getSuccessCount() const { return successCount_; }
  int getFailCount() const { return failCount_; }
  int getTotalPolls() const { return successCount_ + failCount_; }

  /**
   * ルーターインターバル設定（次のルーターまでの待機時間）
   */
  void setRouterInterval(unsigned long ms) { routerIntervalMs_ = ms; }

  /**
   * ポーリングインターバル設定（サイクル完了後の待機時間）
   */
  void setPollInterval(unsigned long ms) { pollIntervalMs_ = ms; }

  /**
   * SSH タイムアウト設定
   */
  void setSshTimeout(unsigned long ms) { sshTimeout_ = ms; }

  /**
   * ルーター情報取得（CelestialGlobe送信用）
   */
  RouterInfo* getRouterInfos() { return routerInfos_; }
  int getRouterInfoCount() const { return routerInfoCount_; }

  /**
   * 最終ポーリング時刻取得
   */
  unsigned long getLastPollTime() const { return lastPollTime_; }

  /**
   * 復旧リスタートが必要か
   * watchdog発火で立つ。呼び出し側でESP.restart()すべき
   */
  bool needsRestart() const { return needsRestart_; }
  String getRestartReason() const { return restartReason_; }

  /**
   * ログ破棄統計（安定性のためにログを捨てた回数）
   * 異常に多い場合は別問題の兆候
   */
  uint32_t getDroppedLogCount() const { return droppedLogCount_; }
  unsigned long getLastLogDropMs() const { return lastLogDropMs_; }
  void resetDroppedLogCount() { droppedLogCount_ = 0; lastLogDropMs_ = 0; }

  /**
   * 外部ミューテックス設定（Serial統一用）
   */
  void setSerialMutex(SemaphoreHandle_t mutex) { externalSerialMutex_ = mutex; }

  // ========================================
  // コールバック
  // ========================================

  /**
   * ルーターポーリング完了時コールバック
   * @param index ルーターインデックス
   * @param success 成功フラグ
   * @param info ルーター情報
   */
  void onRouterPolled(std::function<void(int index, bool success, const RouterInfo& info)> cb) {
    routerPolledCallback_ = cb;
  }

  /**
   * サイクル完了時コールバック
   * @param success 成功数
   * @param total 総数
   */
  void onCycleComplete(std::function<void(int success, int total)> cb) {
    cycleCompleteCallback_ = cb;
  }

private:
  // 設定
  RouterConfig* routers_ = nullptr;
  int* routerCount_ = nullptr;
  unsigned long sshTimeout_ = 90000;
  unsigned long routerIntervalMs_ = 10000;   // ルーター間待機
  unsigned long pollIntervalMs_ = 30000;     // サイクル間待機

  // 状態
  bool enabled_ = true;
  volatile bool sshInProgress_ = false;
  volatile bool sshDone_ = false;
  volatile bool sshSuccess_ = false;
  int currentRouterIndex_ = 0;
  int startIndex_ = 0;                       // サイクル開始インデックス（ランダム）
  int processedCount_ = 0;                   // 今サイクルで処理済み数
  int successCount_ = 0;
  int failCount_ = 0;
  unsigned long sshTaskStartTime_ = 0;
  unsigned long lastPollTime_ = 0;

  // 非ブロッキングタイミング
  unsigned long nextRouterAt_ = 0;           // 次のルーター開始時刻
  unsigned long nextCycleAt_ = 0;            // 次のサイクル開始時刻
  bool cycleInProgress_ = false;

  // 復旧フラグ
  volatile bool needsRestart_ = false;
  String restartReason_;

  // ログ破棄統計
  uint32_t droppedLogCount_ = 0;
  unsigned long lastLogDropMs_ = 0;

  // 収集データ（20スロット固定）
  static const int MAX_ROUTER_INFOS = 20;
  RouterInfo routerInfos_[MAX_ROUTER_INFOS];
  int routerInfoCount_ = 0;

  // タスク
  static const uint32_t SSH_TASK_STACK_SIZE = 32768;  // 32KB
  static const unsigned long SSH_TASK_WATCHDOG_MS = 90000;  // 90秒
  TaskHandle_t sshTaskHandle_ = nullptr;

  // タスクパラメータ（同時に1タスク前提）
  SshTaskParams taskParams_;

  // Serialミューテックス（外部統一 or 内部）
  SemaphoreHandle_t externalSerialMutex_ = nullptr;
  SemaphoreHandle_t internalSerialMutex_ = nullptr;

  // コールバック
  std::function<void(int, bool, const RouterInfo&)> routerPolledCallback_ = nullptr;
  std::function<void(int, int)> cycleCompleteCallback_ = nullptr;

  // SSHタスク関数
  static void sshTaskFunction(void* params);
  void executeSSH(int index);

  // ヘルパー
  void startCycle();
  void startSshTask(int index);
  void processCompletedSsh();
  void handleWatchdog();
  void advanceToNextRouter();
  int getActualRouterIndex(int offset);
  String generateRouterLacisId(const String& mac);

  // パース用ヘルパー
  static String parseIpFromIfconfig(const String& output);
  static String parseSsidFromUci(const String& output, const String& band);
  static int parseClientCount(const String& output);

  // スレッドセーフ出力
  void serialPrintf(const char* format, ...);
  SemaphoreHandle_t getSerialMutex();
};

#endif // SSH_POLLER_IS10_H
