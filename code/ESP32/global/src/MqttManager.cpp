/**
 * MqttManager.cpp
 *
 * Aranea デバイス共通 MQTT クライアント実装
 */

#include "MqttManager.h"
#include <esp_crt_bundle.h>

MqttManager::MqttManager() {}

MqttManager::~MqttManager() {
  stop();
}

bool MqttManager::begin(const String& url, const String& username, const String& password) {
  // 既存クライアントがあれば停止
  if (client_) {
    stop();
  }

  // パラメータチェック
  if (url.length() == 0) {
    Serial.println("[MQTT] No URL configured");
    return false;
  }

  if (username.length() == 0 || password.length() == 0) {
    Serial.println("[MQTT] Missing credentials");
    return false;
  }

  // 接続情報を保存（再接続用）
  url_ = url;
  username_ = username;
  password_ = password;

  Serial.printf("[MQTT] Connecting to %s\n", url_.c_str());
  Serial.printf("[MQTT] Username: %s\n", username_.c_str());

  // ESP-IDF MQTT設定
  esp_mqtt_client_config_t mqttCfg = {};
  mqttCfg.broker.address.uri = url_.c_str();
  mqttCfg.credentials.username = username_.c_str();
  mqttCfg.credentials.authentication.password = password_.c_str();
  mqttCfg.network.timeout_ms = networkTimeoutMs_;
  mqttCfg.session.keepalive = keepAliveSec_;

  // SSL/TLS設定（ESP-IDF証明書バンドル使用）
  mqttCfg.broker.verification.crt_bundle_attach = esp_crt_bundle_attach;

  // クライアント初期化
  client_ = esp_mqtt_client_init(&mqttCfg);
  if (!client_) {
    Serial.println("[MQTT] Failed to init client");
    return false;
  }

  // イベントハンドラ登録（thisポインタを渡す）
  esp_mqtt_client_register_event(client_, MQTT_EVENT_ANY, eventHandler, this);

  // 接続開始
  esp_err_t err = esp_mqtt_client_start(client_);
  if (err != ESP_OK) {
    Serial.printf("[MQTT] Failed to start: %d\n", err);
    esp_mqtt_client_destroy(client_);
    client_ = nullptr;
    return false;
  }

  initialized_ = true;
  return true;
}

void MqttManager::handle() {
  if (!initialized_ || !client_) return;

  // 自動再接続チェック
  if (reconnectIntervalMs_ > 0 && !connected_) {
    unsigned long now = millis();
    if (now - lastReconnectAttempt_ >= reconnectIntervalMs_) {
      Serial.println("[MQTT] Attempting reconnect...");
      reconnect();
      lastReconnectAttempt_ = now;
    }
  }
}

void MqttManager::stop() {
  if (client_) {
    esp_mqtt_client_stop(client_);
    esp_mqtt_client_destroy(client_);
    client_ = nullptr;
  }
  connected_ = false;
  initialized_ = false;
}

int MqttManager::publish(const String& topic, const String& payload, int qos, bool retain) {
  if (!client_ || !connected_) {
    Serial.println("[MQTT] Cannot publish - not connected");
    return -1;
  }

  int msgId = esp_mqtt_client_publish(client_, topic.c_str(), payload.c_str(),
                                       payload.length(), qos, retain ? 1 : 0);
  if (msgId < 0) {
    Serial.printf("[MQTT] Publish failed: %d\n", msgId);
  }
  return msgId;
}

int MqttManager::subscribe(const String& topic, int qos) {
  if (!client_) {
    Serial.println("[MQTT] Cannot subscribe - client not initialized");
    return -1;
  }

  int msgId = esp_mqtt_client_subscribe(client_, topic.c_str(), qos);
  if (msgId >= 0) {
    Serial.printf("[MQTT] Subscribed: %s (qos=%d)\n", topic.c_str(), qos);
  } else {
    Serial.printf("[MQTT] Subscribe failed: %s\n", topic.c_str());
  }
  return msgId;
}

int MqttManager::unsubscribe(const String& topic) {
  if (!client_) return -1;

  int msgId = esp_mqtt_client_unsubscribe(client_, topic.c_str());
  if (msgId >= 0) {
    Serial.printf("[MQTT] Unsubscribed: %s\n", topic.c_str());
  }
  return msgId;
}

void MqttManager::reconnect() {
  if (client_) {
    esp_mqtt_client_reconnect(client_);
  } else if (url_.length() > 0) {
    // クライアントがない場合は再初期化
    begin(url_, username_, password_);
  }
}

// ========================================
// イベントハンドラ（静的関数）
// ========================================
void MqttManager::eventHandler(void* handlerArgs, esp_event_base_t base,
                                int32_t eventId, void* eventData) {
  MqttManager* self = static_cast<MqttManager*>(handlerArgs);
  if (self) {
    self->handleEvent(static_cast<esp_mqtt_event_handle_t>(eventData), eventId);
  }
}

void MqttManager::handleEvent(esp_mqtt_event_handle_t event, int32_t eventId) {
  switch (eventId) {
    case MQTT_EVENT_CONNECTED:
      Serial.println("[MQTT] Connected");
      connected_ = true;
      if (connectedCallback_) {
        connectedCallback_();
      }
      break;

    case MQTT_EVENT_DISCONNECTED:
      Serial.println("[MQTT] Disconnected");
      connected_ = false;
      if (disconnectedCallback_) {
        disconnectedCallback_();
      }
      break;

    case MQTT_EVENT_DATA:
      if (event->topic && event->data && messageCallback_) {
        // トピックとデータをコピー（null終端）
        char* topic = (char*)malloc(event->topic_len + 1);
        char* data = (char*)malloc(event->data_len + 1);

        if (topic && data) {
          memcpy(topic, event->topic, event->topic_len);
          topic[event->topic_len] = '\0';
          memcpy(data, event->data, event->data_len);
          data[event->data_len] = '\0';

          messageCallback_(String(topic), data, event->data_len);

          free(topic);
          free(data);
        } else {
          Serial.println("[MQTT] Memory allocation failed for message");
          if (topic) free(topic);
          if (data) free(data);
        }
      }
      break;

    case MQTT_EVENT_ERROR:
      Serial.println("[MQTT] Error");
      if (event->error_handle) {
        String errorMsg;
        if (event->error_handle->error_type == MQTT_ERROR_TYPE_TCP_TRANSPORT) {
          errorMsg = "TCP error: " + String(event->error_handle->esp_tls_last_esp_err);
          Serial.printf("[MQTT] %s\n", errorMsg.c_str());
        } else if (event->error_handle->error_type == MQTT_ERROR_TYPE_CONNECTION_REFUSED) {
          errorMsg = "Connection refused: " + String(event->error_handle->connect_return_code);
          Serial.printf("[MQTT] %s\n", errorMsg.c_str());
        }
        if (errorCallback_ && errorMsg.length() > 0) {
          errorCallback_(errorMsg);
        }
      }
      break;

    case MQTT_EVENT_SUBSCRIBED:
      Serial.printf("[MQTT] Subscribed, msg_id=%d\n", event->msg_id);
      break;

    case MQTT_EVENT_UNSUBSCRIBED:
      Serial.printf("[MQTT] Unsubscribed, msg_id=%d\n", event->msg_id);
      break;

    case MQTT_EVENT_PUBLISHED:
      Serial.printf("[MQTT] Published, msg_id=%d\n", event->msg_id);
      break;

    default:
      break;
  }
}
