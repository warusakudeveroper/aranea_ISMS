/**
 * SshClient.cpp
 *
 * ESP32用SSHクライアントラッパー実装
 * LibSSH-ESP32ライブラリを使用
 *
 * 注意: LibSSH-ESP32ライブラリのインストールが必要
 * Arduino Library Manager -> "LibSSH-ESP32"
 */

#include "SshClient.h"

// LibSSH-ESP32が利用可能かチェック
#if __has_include("libssh_esp32.h")
  #define SSH_AVAILABLE 1
  #include "libssh_esp32.h"
  #include <libssh/libssh.h>
#else
  #define SSH_AVAILABLE 0
  #warning "LibSSH-ESP32 not installed. SSH functionality will be disabled."
#endif

SshClient::SshClient() : _connected(false), _lastError(""), _session(nullptr), _channel(nullptr) {
#if SSH_AVAILABLE
  // libssh初期化
  libssh_begin();
#endif
}

SshClient::~SshClient() {
  disconnect();
}

SshResult SshClient::connect(const SshConfig& config) {
#if !SSH_AVAILABLE
  _lastError = "LibSSH-ESP32 not installed";
  return SshResult::CONNECT_FAILED;
#else
  if (_connected) {
    disconnect();
  }

  Serial.printf("[SSH] Connecting to %s:%d as %s\n",
    config.host.c_str(), config.port, config.username.c_str());

  // セッション作成
  ssh_session session = ssh_new();
  if (session == NULL) {
    _lastError = "Failed to create SSH session";
    return SshResult::CONNECT_FAILED;
  }

  // オプション設定
  ssh_options_set(session, SSH_OPTIONS_HOST, config.host.c_str());
  int port = config.port;
  ssh_options_set(session, SSH_OPTIONS_PORT, &port);
  ssh_options_set(session, SSH_OPTIONS_USER, config.username.c_str());

  // タイムアウト設定
  long timeout = config.timeout / 1000;  // 秒に変換
  if (timeout <= 0) timeout = 30;
  ssh_options_set(session, SSH_OPTIONS_TIMEOUT, &timeout);

  // 接続
  int rc = ssh_connect(session);
  if (rc != SSH_OK) {
    _lastError = String("Connection failed: ") + ssh_get_error(session);
    Serial.printf("[SSH] %s\n", _lastError.c_str());
    ssh_free(session);
    return SshResult::CONNECT_FAILED;
  }

  // サーバー認証（known_hostsのチェックをスキップ - 組み込みデバイス用）
  // 本番環境ではサーバー公開鍵の検証を推奨

  // パスワード認証
  rc = ssh_userauth_password(session, NULL, config.password.c_str());
  if (rc != SSH_AUTH_SUCCESS) {
    _lastError = String("Authentication failed: ") + ssh_get_error(session);
    Serial.printf("[SSH] %s\n", _lastError.c_str());
    ssh_disconnect(session);
    ssh_free(session);
    return SshResult::AUTH_FAILED;
  }

  _session = session;
  _connected = true;
  Serial.println("[SSH] Connected successfully");

  return SshResult::OK;
#endif
}

void SshClient::disconnect() {
#if SSH_AVAILABLE
  if (_channel != nullptr) {
    ssh_channel channel = (ssh_channel)_channel;
    ssh_channel_send_eof(channel);
    ssh_channel_close(channel);
    ssh_channel_free(channel);
    _channel = nullptr;
  }

  if (_session != nullptr) {
    ssh_session session = (ssh_session)_session;
    ssh_disconnect(session);
    ssh_free(session);
    _session = nullptr;
  }
#endif

  _connected = false;
  Serial.println("[SSH] Disconnected");
}

bool SshClient::isConnected() {
  return _connected;
}

SshExecResult SshClient::exec(const String& command) {
  SshExecResult result;
  result.result = SshResult::NOT_CONNECTED;
  result.output = "";
  result.exitCode = -1;

#if !SSH_AVAILABLE
  result.result = SshResult::EXEC_FAILED;
  _lastError = "LibSSH-ESP32 not installed";
  return result;
#else

  if (!_connected || _session == nullptr) {
    _lastError = "Not connected";
    return result;
  }

  ssh_session session = (ssh_session)_session;

  // チャンネル作成
  ssh_channel channel = ssh_channel_new(session);
  if (channel == NULL) {
    _lastError = "Failed to create channel";
    result.result = SshResult::EXEC_FAILED;
    return result;
  }

  // セッションオープン
  int rc = ssh_channel_open_session(channel);
  if (rc != SSH_OK) {
    _lastError = String("Failed to open channel: ") + ssh_get_error(session);
    ssh_channel_free(channel);
    result.result = SshResult::EXEC_FAILED;
    return result;
  }

  // コマンド実行
  Serial.printf("[SSH] Executing: %s\n", command.c_str());
  rc = ssh_channel_request_exec(channel, command.c_str());
  if (rc != SSH_OK) {
    _lastError = String("Failed to execute: ") + ssh_get_error(session);
    ssh_channel_close(channel);
    ssh_channel_free(channel);
    result.result = SshResult::EXEC_FAILED;
    return result;
  }

  // 出力読み取り
  char buffer[4096];
  int nbytes;

  // stdout読み取り
  while ((nbytes = ssh_channel_read(channel, buffer, sizeof(buffer) - 1, 0)) > 0) {
    buffer[nbytes] = '\0';
    result.output += String(buffer);
  }

  // stderr読み取り
  while ((nbytes = ssh_channel_read(channel, buffer, sizeof(buffer) - 1, 1)) > 0) {
    buffer[nbytes] = '\0';
    result.output += String(buffer);
  }

  // 終了コード取得
  ssh_channel_send_eof(channel);
  ssh_channel_close(channel);
  result.exitCode = ssh_channel_get_exit_status(channel);
  ssh_channel_free(channel);

  result.result = SshResult::OK;
  Serial.printf("[SSH] Command completed, exit code: %d\n", result.exitCode);

  return result;
#endif
}

SshExecResult SshClient::execMultiple(const String& commands) {
  // 複数コマンドをセミコロンで連結して実行
  String combined = commands;
  combined.replace("\n", "; ");
  return exec(combined);
}

String SshClient::getLastError() {
  return _lastError;
}
