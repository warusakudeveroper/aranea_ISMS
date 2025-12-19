/**
 * RouterTypes.h
 *
 * IS10用ルーター関連の共通型定義
 * is10.ino と HttpManagerIs10 の両方で使用
 */

#ifndef ROUTER_TYPES_H
#define ROUTER_TYPES_H

#include <Arduino.h>

// 最大ルーター数
#define MAX_ROUTERS 20

// ルーターOSタイプ
enum class RouterOsType {
  OPENWRT = 0,   // OpenWrt (uci, br-lan, /tmp/dhcp.leases)
  ASUSWRT = 1    // ASUSWRT (nvram, br0, /var/lib/misc/dnsmasq.leases)
};

// ルーター設定構造体
struct RouterConfig {
  String rid;           // Resource ID
  String ipAddr;        // IPアドレス
  String publicKey;     // SSH公開鍵（認証用）
  int sshPort;          // SSHポート
  String username;      // ユーザー名
  String password;      // パスワード
  bool enabled;         // 有効フラグ
  RouterOsType osType;  // OSタイプ（OpenWrt / ASUSWRT）
};

#endif // ROUTER_TYPES_H
