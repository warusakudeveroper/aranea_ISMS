#include "HttpOtaManager.h"
#include <ArduinoJson.h>

void HttpOtaManager::begin(WebServer* server, const String& authToken) {
  server_ = server;
  authToken_ = authToken;

  // エンドポイント登録
  server_->on("/api/ota/status", HTTP_GET, [this]() { handleStatus(); });
  server_->on("/api/ota/info", HTTP_GET, [this]() { handleInfo(); });
  server_->on("/api/ota/update", HTTP_POST, [this]() { handleUpdate(); }, [this]() { handleUpload(); });
  server_->on("/api/ota/rollback", HTTP_POST, [this]() { handleRollback(); });
  server_->on("/api/ota/confirm", HTTP_POST, [this]() { handleConfirm(); });

  Serial.println("[HTTP-OTA] Endpoints registered");
}

bool HttpOtaManager::checkAuth() {
  if (authToken_.length() == 0) return true;

  // Authorizationヘッダーまたはtokenパラメータをチェック
  if (server_->hasHeader("Authorization")) {
    String auth = server_->header("Authorization");
    if (auth.startsWith("Bearer ")) {
      auth = auth.substring(7);
    }
    if (auth == authToken_) return true;
  }

  if (server_->hasArg("token")) {
    if (server_->arg("token") == authToken_) return true;
  }

  server_->send(401, "application/json", "{\"error\":\"Unauthorized\"}");
  return false;
}

void HttpOtaManager::handleStatus() {
  if (!checkAuth()) return;

  StaticJsonDocument<256> doc;
  doc["status"] = statusToString(status_);
  doc["progress"] = progress_;

  if (status_ == HttpOtaStatus::ERROR) {
    doc["error"] = errorMessage_;
  }

  if (status_ == HttpOtaStatus::IN_PROGRESS) {
    doc["uploaded"] = uploadedSize_;
    doc["total"] = totalSize_;
  }

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpOtaManager::handleInfo() {
  if (!checkAuth()) return;

  HttpOtaInfo info = getOtaInfo();

  StaticJsonDocument<512> doc;
  doc["running_partition"] = info.runningPartition;
  doc["next_partition"] = info.nextPartition;
  doc["can_rollback"] = info.canRollback;
  doc["pending_verify"] = info.pendingVerify;
  doc["partition_size"] = info.partitionSize;

  String response;
  serializeJson(doc, response);
  server_->send(200, "application/json", response);
}

void HttpOtaManager::handleUpdate() {
  if (!checkAuth()) return;

  if (Update.hasError()) {
    setError(Update.errorString());
    StaticJsonDocument<128> doc;
    doc["ok"] = false;
    doc["error"] = errorMessage_;
    String response;
    serializeJson(doc, response);
    server_->send(400, "application/json", response);
  } else {
    status_ = HttpOtaStatus::SUCCESS;
    progress_ = 100;

    StaticJsonDocument<128> doc;
    doc["ok"] = true;
    doc["message"] = "Update successful, rebooting...";
    String response;
    serializeJson(doc, response);
    server_->send(200, "application/json", response);

    if (onEndCallback_) onEndCallback_();

    delay(500);
    ESP.restart();
  }
}

void HttpOtaManager::handleUpload() {
  HTTPUpload& upload = server_->upload();

  if (upload.status == UPLOAD_FILE_START) {
    Serial.printf("[HTTP-OTA] Update: %s\n", upload.filename.c_str());

    status_ = HttpOtaStatus::IN_PROGRESS;
    progress_ = 0;
    uploadedSize_ = 0;
    totalSize_ = upload.totalSize > 0 ? upload.totalSize : 0;
    errorMessage_ = "";

    if (onStartCallback_) onStartCallback_();

    // Update開始
    if (!Update.begin(UPDATE_SIZE_UNKNOWN, U_FLASH)) {
      setError(Update.errorString());
      return;
    }

  } else if (upload.status == UPLOAD_FILE_WRITE) {
    // データ書き込み
    if (Update.write(upload.buf, upload.currentSize) != upload.currentSize) {
      setError(Update.errorString());
      return;
    }

    uploadedSize_ += upload.currentSize;

    // 進捗計算
    if (totalSize_ > 0) {
      progress_ = (uploadedSize_ * 100) / totalSize_;
    } else {
      // totalSizeが不明な場合は推定（1.5MB想定）
      progress_ = min(99, (int)(uploadedSize_ * 100 / 1500000));
    }

    if (onProgressCallback_) onProgressCallback_(progress_);

    // 進捗ログ（10%ごと）
    static int lastLoggedProgress = 0;
    if (progress_ / 10 > lastLoggedProgress / 10) {
      Serial.printf("[HTTP-OTA] Progress: %d%%\n", progress_);
      lastLoggedProgress = progress_;
    }

  } else if (upload.status == UPLOAD_FILE_END) {
    // Update終了
    if (Update.end(true)) {
      Serial.printf("[HTTP-OTA] Update Success: %u bytes\n", upload.totalSize);
      progress_ = 100;
    } else {
      setError(Update.errorString());
    }

  } else if (upload.status == UPLOAD_FILE_ABORTED) {
    Update.abort();
    setError("Upload aborted");
  }
}

void HttpOtaManager::handleRollback() {
  if (!checkAuth()) return;

  const esp_partition_t* running = esp_ota_get_running_partition();
  const esp_partition_t* otadata = esp_partition_find_first(ESP_PARTITION_TYPE_DATA, ESP_PARTITION_SUBTYPE_DATA_OTA, NULL);

  if (!otadata) {
    server_->send(400, "application/json", "{\"ok\":false,\"error\":\"No OTA data partition\"}");
    return;
  }

  // ロールバック可能かチェック
  esp_ota_img_states_t state;
  const esp_partition_t* nextUpdate = esp_ota_get_next_update_partition(NULL);
  if (nextUpdate && esp_ota_get_state_partition(nextUpdate, &state) == ESP_OK) {
    if (state == ESP_OTA_IMG_VALID || state == ESP_OTA_IMG_UNDEFINED) {
      // ロールバック実行
      esp_err_t err = esp_ota_set_boot_partition(nextUpdate);
      if (err == ESP_OK) {
        StaticJsonDocument<128> doc;
        doc["ok"] = true;
        doc["message"] = "Rollback successful, rebooting...";
        String response;
        serializeJson(doc, response);
        server_->send(200, "application/json", response);

        delay(500);
        ESP.restart();
        return;
      }
    }
  }

  server_->send(400, "application/json", "{\"ok\":false,\"error\":\"Rollback not available\"}");
}

void HttpOtaManager::handleConfirm() {
  if (!checkAuth()) return;

  // 現在のパーティションを有効としてマーク
  esp_err_t err = esp_ota_mark_app_valid_cancel_rollback();

  StaticJsonDocument<128> doc;
  if (err == ESP_OK) {
    doc["ok"] = true;
    doc["message"] = "Firmware confirmed as valid";
  } else {
    doc["ok"] = false;
    doc["error"] = "Failed to confirm firmware";
  }

  String response;
  serializeJson(doc, response);
  server_->send(err == ESP_OK ? 200 : 400, "application/json", response);
}

HttpOtaInfo HttpOtaManager::getOtaInfo() {
  HttpOtaInfo info;

  const esp_partition_t* running = esp_ota_get_running_partition();
  const esp_partition_t* nextUpdate = esp_ota_get_next_update_partition(NULL);

  if (running) {
    info.runningPartition = running->label;
    info.partitionSize = running->size;
  }

  if (nextUpdate) {
    info.nextPartition = nextUpdate->label;
  }

  // ロールバック可能かチェック
  info.canRollback = false;
  if (nextUpdate) {
    esp_ota_img_states_t state;
    if (esp_ota_get_state_partition(nextUpdate, &state) == ESP_OK) {
      info.canRollback = (state == ESP_OTA_IMG_VALID);
    }
  }

  // pending_verify チェック
  info.pendingVerify = false;
  if (running) {
    esp_ota_img_states_t state;
    if (esp_ota_get_state_partition(running, &state) == ESP_OK) {
      info.pendingVerify = (state == ESP_OTA_IMG_PENDING_VERIFY);
    }
  }

  return info;
}

String HttpOtaManager::statusToString(HttpOtaStatus status) {
  switch (status) {
    case HttpOtaStatus::IDLE:        return "idle";
    case HttpOtaStatus::IN_PROGRESS: return "in_progress";
    case HttpOtaStatus::SUCCESS:     return "success";
    case HttpOtaStatus::ERROR:       return "error";
    default:                         return "unknown";
  }
}

void HttpOtaManager::setError(const String& msg) {
  status_ = HttpOtaStatus::ERROR;
  errorMessage_ = msg;
  Serial.printf("[HTTP-OTA] Error: %s\n", msg.c_str());
  if (onErrorCallback_) onErrorCallback_(msg);
}
