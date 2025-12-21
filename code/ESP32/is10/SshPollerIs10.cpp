/**
 * SshPollerIs10.cpp
 *
 * IS10 ルーターSSH巡回ポーラー実装
 *
 * P0対応改善:
 * - delay()撤去 → nextAt方式（非ブロッキング）
 * - ランダム開始インデックス（特定ルーターでのハング→再起動ループ防止）
 * - タスクにindex固定渡し（競合防止）
 * - watchdog発火時はrestart flag（タスク積み上げ防止）
 */

#include "SshPollerIs10.h"
#include "libssh_esp32.h"
#include <libssh/libssh.h>
#include <stdarg.h>

// ========================================
// グローバル変数（SSHタスクからアクセス用）
// タスクパラメータ渡しは不安定なためグローバル方式を維持
// ========================================
static SshPollerIs10* gPoller = nullptr;

SshPollerIs10::SshPollerIs10() {}

SshPollerIs10::~SshPollerIs10() {
  if (internalSerialMutex_) {
    vSemaphoreDelete(internalSerialMutex_);
  }
}

void SshPollerIs10::begin(RouterConfig* routers, int* routerCount, unsigned long sshTimeout) {
  routers_ = routers;
  routerCount_ = routerCount;
  sshTimeout_ = sshTimeout;

  // 内部Serialミューテックス作成（外部が設定されていない場合に使用）
  internalSerialMutex_ = xSemaphoreCreateMutex();
  if (!internalSerialMutex_) {
    Serial.println("[SSH-POLLER] Failed to create internal mutex");
  }

  // libssh初期化（1回のみ）
  libssh_begin();
  Serial.println("[SSH-POLLER] libssh initialized");

  // 乱数シード初期化（ランダム開始インデックス用）
  randomSeed(esp_random());

  // グローバル参照設定（SSHタスクからアクセス用）
  gPoller = this;

  Serial.println("[SSH-POLLER] Initialized");
}

SemaphoreHandle_t SshPollerIs10::getSerialMutex() {
  return externalSerialMutex_ ? externalSerialMutex_ : internalSerialMutex_;
}

void SshPollerIs10::handle() {
  if (!enabled_ || !routers_ || !routerCount_ || *routerCount_ == 0) {
    return;
  }

  // 復旧リスタートが必要な場合は何もしない（呼び出し側でESP.restart()）
  if (needsRestart_) {
    return;
  }

  unsigned long now = millis();

  // ウォッチドッグチェック
  handleWatchdog();

  // SSH完了チェック
  if (sshDone_ && sshInProgress_) {
    processCompletedSsh();
  }

  // サイクル開始判定
  if (!cycleInProgress_ && !sshInProgress_) {
    if (now >= nextCycleAt_) {
      startCycle();
    }
    return;
  }

  // 次のルーター開始判定（非ブロッキング）
  if (cycleInProgress_ && !sshInProgress_) {
    if (now >= nextRouterAt_) {
      // ヒープチェック
      uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
      uint32_t requiredMemory = SSH_TASK_STACK_SIZE + 20000;

      if (largestBlock < requiredMemory) {
        serialPrintf("[SSH-POLLER] CRITICAL: Skipping - insufficient memory (%u < %u)\n",
                     largestBlock, requiredMemory);
        // サイクル中断
        cycleInProgress_ = false;
        nextCycleAt_ = now + pollIntervalMs_;
        successCount_ = 0;
        failCount_ = 0;
        processedCount_ = 0;
      } else {
        // 次のルーターを開始
        int actualIndex = getActualRouterIndex(processedCount_);
        startSshTask(actualIndex);
      }
    }
  }
}

// ========================================
// サイクル開始（ランダム開始インデックス）
// ========================================
void SshPollerIs10::startCycle() {
  if (*routerCount_ == 0) return;

  // ランダム開始インデックス（特定ルーターでのハング→再起動ループ防止）
  startIndex_ = random(0, *routerCount_);
  processedCount_ = 0;
  successCount_ = 0;
  failCount_ = 0;
  cycleInProgress_ = true;
  nextRouterAt_ = millis();  // 即座に開始

  serialPrintf("\n[SSH-POLLER] Starting cycle: %d routers, startIndex=%d\n",
               *routerCount_, startIndex_);
}

// ========================================
// 実際のルーターインデックス取得（ラップアラウンド）
// ========================================
int SshPollerIs10::getActualRouterIndex(int offset) {
  return (startIndex_ + offset) % *routerCount_;
}

// ========================================
// ウォッチドッグ処理（改善版）
// ========================================
void SshPollerIs10::handleWatchdog() {
  if (!sshInProgress_ || sshDone_) return;

  unsigned long now = millis();
  if (now - sshTaskStartTime_ >= SSH_TASK_WATCHDOG_MS) {
    serialPrintf("[SSH-POLLER] WATCHDOG: Timeout after %lu ms\n", now - sshTaskStartTime_);
    serialPrintf("[SSH-POLLER] Router %d/%d (actual=%d) - marking for restart\n",
                 processedCount_ + 1, *routerCount_, currentRouterIndex_);

    // 復旧フラグを立てる（タスク積み上げ防止）
    // 呼び出し側でESP.restart()を実行すべき
    needsRestart_ = true;
    restartReason_ = "SSH watchdog timeout at router " + String(currentRouterIndex_);

    // 状態リセット（次の処理を止める）
    sshInProgress_ = false;
    cycleInProgress_ = false;

    serialPrintf("[SSH-POLLER] needsRestart=true - caller should ESP.restart()\n");
  }
}

// ========================================
// SSH完了処理（非ブロッキング）
// ========================================
void SshPollerIs10::processCompletedSsh() {
  sshInProgress_ = false;

  uint32_t freeHeap = ESP.getFreeHeap();
  uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);

  if (sshSuccess_) {
    serialPrintf("[SSH-POLLER] Router %d/%d (actual=%d) SUCCESS (heap=%u, largest=%u)\n",
                 processedCount_ + 1, *routerCount_, currentRouterIndex_, freeHeap, largestBlock);
    successCount_++;
  } else {
    serialPrintf("[SSH-POLLER] Router %d/%d (actual=%d) FAILED (heap=%u, largest=%u)\n",
                 processedCount_ + 1, *routerCount_, currentRouterIndex_, freeHeap, largestBlock);
    failCount_++;
  }

  // コールバック呼び出し
  if (routerPolledCallback_) {
    routerPolledCallback_(currentRouterIndex_, sshSuccess_, routerInfos_[currentRouterIndex_]);
  }

  // ヒープ警告
  if (largestBlock < SSH_TASK_STACK_SIZE + 10000) {
    serialPrintf("[SSH-POLLER] WARNING: Low memory! largest=%u\n", largestBlock);
  }

  sshDone_ = false;

  // 次のルーターへ進む（delay不使用）
  advanceToNextRouter();
}

// ========================================
// 次のルーターへ進む
// ========================================
void SshPollerIs10::advanceToNextRouter() {
  processedCount_++;

  if (processedCount_ >= *routerCount_) {
    // サイクル完了
    uint32_t finalHeap = ESP.getFreeHeap();
    uint32_t finalLargest = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);

    serialPrintf("\n[SSH-POLLER] COMPLETE: %d/%d success\n", successCount_, *routerCount_);
    serialPrintf("[SSH-POLLER] heap=%u, largest=%u\n\n", finalHeap, finalLargest);

    lastPollTime_ = millis();
    routerInfoCount_ = *routerCount_;

    if (cycleCompleteCallback_) {
      cycleCompleteCallback_(successCount_, *routerCount_);
    }

    cycleInProgress_ = false;
    nextCycleAt_ = millis() + pollIntervalMs_;
    processedCount_ = 0;
    successCount_ = 0;
    failCount_ = 0;
  } else {
    // 次のルーターまで待機（非ブロッキング）
    nextRouterAt_ = millis() + routerIntervalMs_;
  }
}

// ========================================
// SSHタスク開始（index固定渡し）
// ========================================
void SshPollerIs10::startSshTask(int index) {
  if (index < 0 || index >= *routerCount_) return;

  RouterConfig& router = routers_[index];
  if (!router.enabled || router.ipAddr.length() == 0) {
    // 無効なルーター → スキップして次へ
    advanceToNextRouter();
    return;
  }

  // フラグリセット
  sshDone_ = false;
  sshSuccess_ = false;
  sshInProgress_ = true;
  sshTaskStartTime_ = millis();
  currentRouterIndex_ = index;

  serialPrintf("[SSH-POLLER] Starting SSH for Router %d/%d (actual=%d)\n",
               processedCount_ + 1, *routerCount_, index);

  // SSHタスク起動（gPoller経由でアクセス、index不使用・currentRouterIndex_を使用）
  BaseType_t xReturned = xTaskCreatePinnedToCore(
    sshTaskFunction,
    "ssh_poll",
    SSH_TASK_STACK_SIZE,
    NULL,  // gPoller経由なのでパラメータ不要
    tskIDLE_PRIORITY + 1,
    &sshTaskHandle_,
    1  // Core 1
  );

  if (xReturned != pdPASS) {
    serialPrintf("[SSH-POLLER] ERROR: Failed to create SSH task\n");
    sshInProgress_ = false;
    sshSuccess_ = false;
    sshDone_ = true;  // 失敗として処理
  }
}

// ========================================
// SSHタスク関数（静的）
// gPoller経由でクラスインスタンスにアクセス
// ========================================
void SshPollerIs10::sshTaskFunction(void* params) {
  if (gPoller) {
    gPoller->executeSSH(gPoller->currentRouterIndex_);
  }
  vTaskDelete(NULL);
}

void SshPollerIs10::executeSSH(int index) {
  RouterConfig& router = routers_[index];
  bool isAsuswrt = (router.osType == RouterOsType::ASUSWRT);

  serialPrintf("[SSH] Router %d/%d: %s (%s)\n",
               processedCount_ + 1, *routerCount_, router.rid.c_str(), router.ipAddr.c_str());

  // RouterInfo初期化
  RouterInfo& info = routerInfos_[index];
  info.rid = router.rid;
  info.routerMac = "";
  info.wanIp = "";
  info.lanIp = "";
  info.ssid24 = "";
  info.ssid50 = "";
  info.clientCount = 0;
  info.online = false;
  info.lastUpdate = millis();

  bool connectedOk = false;
  bool dataOk = false;

  // SSHセッション作成
  ssh_session session = ssh_new();
  if (session == NULL) {
    serialPrintf("[SSH] Failed to create session\n");
    sshSuccess_ = false;
    sshDone_ = true;
    return;
  }

  // SSH接続設定
  ssh_options_set(session, SSH_OPTIONS_HOST, router.ipAddr.c_str());
  unsigned int port = router.sshPort;
  ssh_options_set(session, SSH_OPTIONS_PORT, &port);
  ssh_options_set(session, SSH_OPTIONS_USER, router.username.c_str());
  long timeout = sshTimeout_ / 1000;  // 秒単位
  ssh_options_set(session, SSH_OPTIONS_TIMEOUT, &timeout);

  // 接続
  int rc = ssh_connect(session);
  if (rc != SSH_OK) {
    serialPrintf("[SSH] Connect failed: %s\n", ssh_get_error(session));
    ssh_free(session);
    sshSuccess_ = false;
    sshDone_ = true;
    return;
  }

  // 認証
  rc = ssh_userauth_password(session, NULL, router.password.c_str());
  if (rc != SSH_AUTH_SUCCESS) {
    serialPrintf("[SSH] Auth failed: %s\n", ssh_get_error(session));
    ssh_disconnect(session);
    ssh_free(session);
    sshSuccess_ = false;
    sshDone_ = true;
    return;
  }

  connectedOk = true;
  info.online = true;
  serialPrintf("[SSH] Auth success\n");

  // ========================================
  // コマンド実行
  // ========================================

  // WAN IP取得
  ssh_channel channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ? "nvram get wan0_ipaddr" : "ifconfig eth0 2>/dev/null";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          if (isAsuswrt) {
            String val = String(buf);
            val.trim();
            info.wanIp = val;
          } else {
            info.wanIp = parseIpFromIfconfig(String(buf));
          }
          if (info.wanIp.length() > 0) {
            serialPrintf("[SSH] WAN: %s\n", info.wanIp.c_str());
            dataOk = true;
          }
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  // LAN IP取得
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ? "nvram get lan_ipaddr" : "ifconfig br-lan 2>/dev/null";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          if (isAsuswrt) {
            String val = String(buf);
            val.trim();
            info.lanIp = val;
          } else {
            info.lanIp = parseIpFromIfconfig(String(buf));
          }
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  // 2.4GHz SSID取得
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ? "nvram get wl0_ssid" : "uci get wireless.default_radio0.ssid 2>/dev/null";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          String val = String(buf);
          val.trim();
          info.ssid24 = val;
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  // 5GHz SSID取得
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ? "nvram get wl1_ssid" : "uci get wireless.default_radio1.ssid 2>/dev/null";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[256];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          String val = String(buf);
          val.trim();
          info.ssid50 = val;
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  // クライアント数取得
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ?
        "cat /var/lib/misc/dnsmasq.leases 2>/dev/null | wc -l" :
        "cat /tmp/dhcp.leases 2>/dev/null | wc -l";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[64];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          info.clientCount = atoi(buf);
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  // MACアドレス取得
  channel = ssh_channel_new(session);
  if (channel) {
    if (ssh_channel_open_session(channel) == SSH_OK) {
      const char* cmd = isAsuswrt ?
        "cat /sys/class/net/br0/address 2>/dev/null" :
        "cat /sys/class/net/br-lan/address 2>/dev/null";
      if (ssh_channel_request_exec(channel, cmd) == SSH_OK) {
        char buf[64];
        int n = ssh_channel_read(channel, buf, sizeof(buf) - 1, 0);
        if (n > 0) {
          buf[n] = '\0';
          String mac = String(buf);
          mac.trim();
          // 正規化（大文字12HEX）
          mac.replace(":", "");
          mac.replace("-", "");
          mac.toUpperCase();
          info.routerMac = mac;
          info.lacisId = generateRouterLacisId(mac);
        }
      }
      ssh_channel_send_eof(channel);
      ssh_channel_close(channel);
    }
    ssh_channel_free(channel);
  }

  ssh_disconnect(session);
  ssh_free(session);

  info.lastUpdate = millis();

  // 成功判定: connectedOk（SSH認証成功）を優先
  sshSuccess_ = connectedOk;
  serialPrintf("[SSH] Done: connectedOk=%d, dataOk=%d\n", connectedOk, dataOk);

  sshDone_ = true;
}

// ========================================
// ヘルパー関数
// ========================================

String SshPollerIs10::generateRouterLacisId(const String& mac) {
  if (mac.length() != 12) return "";
  // フォーマット: 4 + 000 + MAC12桁 + 0000
  return "4000" + mac + "0000";
}

String SshPollerIs10::parseIpFromIfconfig(const String& output) {
  int pos = output.indexOf("inet ");
  if (pos < 0) pos = output.indexOf("inet:");
  if (pos < 0) return "";

  String sub = output.substring(pos + 5);
  sub.replace("addr:", "");
  sub.trim();

  int spacePos = sub.indexOf(' ');
  if (spacePos > 0) {
    sub = sub.substring(0, spacePos);
  }
  return sub;
}

String SshPollerIs10::parseSsidFromUci(const String& output, const String& band) {
  String search = band + ".ssid='";
  int pos = output.indexOf(search);
  if (pos < 0) {
    search = band + ".ssid=";
    pos = output.indexOf(search);
  }
  if (pos < 0) return "";

  int start = pos + search.length();
  int end = output.indexOf("'", start);
  if (end < 0) end = output.indexOf("\n", start);
  if (end < 0) end = output.length();

  return output.substring(start, end);
}

int SshPollerIs10::parseClientCount(const String& output) {
  int count = 0;
  int pos = 0;
  while ((pos = output.indexOf('\n', pos)) >= 0) {
    count++;
    pos++;
  }
  return count;
}

void SshPollerIs10::serialPrintf(const char* format, ...) {
  va_list args;
  va_start(args, format);

  SemaphoreHandle_t mutex = getSerialMutex();
  if (mutex) xSemaphoreTake(mutex, portMAX_DELAY);

  char buf[256];
  vsnprintf(buf, sizeof(buf), format, args);
  Serial.print(buf);

  if (mutex) xSemaphoreGive(mutex);

  va_end(args);
}
