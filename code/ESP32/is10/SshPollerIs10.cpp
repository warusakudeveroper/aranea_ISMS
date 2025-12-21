/**
 * SshPollerIs10.cpp
 *
 * IS10 ルーターSSH巡回ポーラー実装
 */

#include "SshPollerIs10.h"
#include "libssh_esp32.h"
#include <libssh/libssh.h>
#include <stdarg.h>

// ========================================
// グローバル変数（SSHタスクからアクセス用）
// ========================================
static SshPollerIs10* gPoller = nullptr;

SshPollerIs10::SshPollerIs10() {}

SshPollerIs10::~SshPollerIs10() {
  if (serialMutex_) {
    vSemaphoreDelete(serialMutex_);
  }
}

void SshPollerIs10::begin(RouterConfig* routers, int* routerCount, unsigned long sshTimeout) {
  routers_ = routers;
  routerCount_ = routerCount;
  sshTimeout_ = sshTimeout;

  // Serialミューテックス作成
  serialMutex_ = xSemaphoreCreateMutex();
  if (!serialMutex_) {
    Serial.println("[SSH-POLLER] Failed to create mutex");
  }

  // libssh初期化（1回のみ）
  libssh_begin();
  Serial.println("[SSH-POLLER] libssh initialized");

  // グローバル参照設定
  gPoller = this;

  lastRouterPoll_ = millis();
}

void SshPollerIs10::handle() {
  if (!enabled_ || !routers_ || !routerCount_ || *routerCount_ == 0) {
    return;
  }

  unsigned long now = millis();

  // ウォッチドッグチェック
  handleWatchdog();

  // SSH完了チェック
  if (sshDone_ && sshInProgress_) {
    processCompletedSsh();
  }

  // 新しいSSH開始
  if (!sshInProgress_) {
    // 最初のルーターまたはインターバル経過後
    if (currentRouterIndex_ == 0 || (now - lastRouterPoll_ >= pollIntervalMs_)) {
      // ヒープチェック
      uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
      uint32_t requiredMemory = SSH_TASK_STACK_SIZE + 20000;

      if (largestBlock < requiredMemory) {
        serialPrintf("[SSH-POLLER] CRITICAL: Skipping - insufficient memory (%u < %u)\n",
                     largestBlock, requiredMemory);
        // サイクル中断
        currentRouterIndex_ = 0;
        successCount_ = 0;
        failCount_ = 0;
        lastRouterPoll_ = now;
      } else {
        startSshTask(currentRouterIndex_);
        if (currentRouterIndex_ == 0) {
          lastRouterPoll_ = now;
        }
      }
    }
  }
}

void SshPollerIs10::handleWatchdog() {
  if (!sshInProgress_ || sshDone_) return;

  unsigned long now = millis();
  if (now - sshTaskStartTime_ >= SSH_TASK_WATCHDOG_MS) {
    serialPrintf("[SSH-POLLER] Watchdog timeout after %lu ms\n", now - sshTaskStartTime_);
    serialPrintf("[SSH-POLLER] Router %d/%d marked as FAILED\n",
                 currentRouterIndex_ + 1, *routerCount_);

    // 強制的に失敗として処理
    sshInProgress_ = false;
    sshDone_ = true;
    sshSuccess_ = false;
    failCount_++;

    // 次のルーターへ
    currentRouterIndex_++;
    if (currentRouterIndex_ >= *routerCount_) {
      // サイクル完了
      lastPollTime_ = millis();
      routerInfoCount_ = *routerCount_;

      if (cycleCompleteCallback_) {
        cycleCompleteCallback_(successCount_, *routerCount_);
      }

      currentRouterIndex_ = 0;
      successCount_ = 0;
      failCount_ = 0;
      lastRouterPoll_ = millis();
    }

    sshDone_ = false;
  }
}

void SshPollerIs10::processCompletedSsh() {
  sshInProgress_ = false;

  uint32_t freeHeap = ESP.getFreeHeap();
  uint32_t largestBlock = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);

  if (sshSuccess_) {
    serialPrintf("[SSH-POLLER] Router %d/%d SUCCESS (heap=%u, largest=%u)\n",
                 currentRouterIndex_ + 1, *routerCount_, freeHeap, largestBlock);
    successCount_++;
  } else {
    serialPrintf("[SSH-POLLER] Router %d/%d FAILED (heap=%u, largest=%u)\n",
                 currentRouterIndex_ + 1, *routerCount_, freeHeap, largestBlock);
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

  // 次のルーターへ（待機時間を挿入）
  delay(routerIntervalMs_);

  currentRouterIndex_++;
  if (currentRouterIndex_ >= *routerCount_) {
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

    currentRouterIndex_ = 0;
    successCount_ = 0;
    failCount_ = 0;
    lastRouterPoll_ = millis();
  }
}

void SshPollerIs10::startSshTask(int index) {
  if (index < 0 || index >= *routerCount_) return;

  RouterConfig& router = routers_[index];
  if (!router.enabled || router.ipAddr.length() == 0) {
    return;
  }

  // フラグリセット
  sshDone_ = false;
  sshSuccess_ = false;
  sshInProgress_ = true;
  sshTaskStartTime_ = millis();

  serialPrintf("[SSH-POLLER] Starting SSH for Router %d/%d\n", index + 1, *routerCount_);

  // SSHタスク起動
  BaseType_t xReturned = xTaskCreatePinnedToCore(
    sshTaskFunction,
    "ssh_poll",
    SSH_TASK_STACK_SIZE,
    this,
    tskIDLE_PRIORITY + 1,
    &sshTaskHandle_,
    1  // Core 1
  );

  if (xReturned != pdPASS) {
    serialPrintf("[SSH-POLLER] ERROR: Failed to create SSH task\n");
    sshInProgress_ = false;
  }
}

// ========================================
// SSHタスク関数（静的）
// ========================================
void SshPollerIs10::sshTaskFunction(void* params) {
  SshPollerIs10* self = static_cast<SshPollerIs10*>(params);
  if (self) {
    self->executeSSH(self->currentRouterIndex_);
  }
  vTaskDelete(NULL);
}

void SshPollerIs10::executeSSH(int index) {
  RouterConfig& router = routers_[index];
  bool isAsuswrt = (router.osType == RouterOsType::ASUSWRT);

  serialPrintf("[SSH] Router %d/%d: %s (%s)\n",
               index + 1, *routerCount_, router.rid.c_str(), router.ipAddr.c_str());

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

  if (serialMutex_) xSemaphoreTake(serialMutex_, portMAX_DELAY);

  char buf[256];
  vsnprintf(buf, sizeof(buf), format, args);
  Serial.print(buf);

  if (serialMutex_) xSemaphoreGive(serialMutex_);

  va_end(args);
}
