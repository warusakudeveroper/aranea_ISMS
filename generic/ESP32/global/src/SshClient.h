/**
 * SshClient.h
 *
 * ESP32用SSHクライアントラッパー
 * LibSSH-ESP32ライブラリを使用
 *
 * 使用前に Arduino Library Manager から "LibSSH-ESP32" をインストール
 */

#ifndef SSH_CLIENT_H
#define SSH_CLIENT_H

#include <Arduino.h>

// SSH接続結果
enum class SshResult {
  OK,
  CONNECT_FAILED,
  AUTH_FAILED,
  EXEC_FAILED,
  TIMEOUT,
  NOT_CONNECTED
};

// SSH接続設定
struct SshConfig {
  String host;
  int port;
  String username;
  String password;
  String publicKey;       // 公開鍵認証用（オプション）
  unsigned long timeout;  // タイムアウト（ms）
};

// コマンド実行結果
struct SshExecResult {
  SshResult result;
  String output;
  int exitCode;
};

/**
 * SshClient - SSHクライアントクラス
 *
 * 重要: コンストラクタではlibsshを初期化しません。
 *       必ず setup() 後、WiFi接続完了後に begin() を呼んでください。
 *
 * 使用例:
 *   SshClient ssh;
 *   // WiFi接続後...
 *   ssh.begin();  // libssh初期化
 *   SshConfig config = {"192.168.1.1", 22, "admin", "password", "", 30000};
 *   if (ssh.connect(config) == SshResult::OK) {
 *     SshExecResult result = ssh.exec("uname -a");
 *     Serial.println(result.output);
 *     ssh.disconnect();
 *   }
 */
class SshClient {
public:
  SshClient();
  ~SshClient();

  /**
   * libsshライブラリを初期化
   * 必ず setup() 後、WiFi接続完了後に呼ぶこと
   * コンストラクタでは呼ばないこと（グローバル変数の初期化順序問題回避）
   */
  void begin();

  // 初期化済みか確認
  bool isInitialized();

  // 接続
  SshResult connect(const SshConfig& config);

  // 切断
  void disconnect();

  // 接続状態確認
  bool isConnected();

  // コマンド実行
  SshExecResult exec(const String& command);

  // 複数コマンド実行（改行区切り）
  SshExecResult execMultiple(const String& commands);

  // 最後のエラーメッセージ
  String getLastError();

private:
  bool _connected;
  String _lastError;
  void* _session;  // libssh session handle
  void* _channel;  // libssh channel handle
};

#endif // SSH_CLIENT_H
