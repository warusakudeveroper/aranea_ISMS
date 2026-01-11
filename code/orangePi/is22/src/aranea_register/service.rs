//! AraneaRegister Service
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## 責務
//! - is22デバイスのaraneaDeviceGateへの登録
//! - CIC取得・永続化
//! - 登録状態管理
//!
//! ## 参照実装
//! - ESP32版: code/ESP32/global/src/AraneaRegister.cpp

use super::{
    lacis_id::{get_primary_mac_address, try_generate_lacis_id},
    repository::AraneaRegistrationRepository,
    types::*,
};
use crate::config_store::ConfigStore;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use sqlx::MySqlPool;
use std::sync::Arc;
use std::time::Duration;

/// AraneaRegister Service
///
/// ## 使用例
/// ```rust,ignore
/// let service = AraneaRegisterService::new(
///     "https://api.example.com/api/araneaDeviceGate".to_string(),
///     pool,
///     config_store,
/// );
///
/// let result = service.register_device(RegisterRequest {
///     tenant_primary_auth: TenantPrimaryAuth { ... },
///     tid: "T2025...".to_string(),
/// }).await?;
/// ```
pub struct AraneaRegisterService {
    gate_url: String,
    http_client: Client,
    repository: AraneaRegistrationRepository,
    config_store: Arc<ConfigStore>,
}

impl AraneaRegisterService {
    /// Create new AraneaRegisterService
    ///
    /// # Arguments
    /// * `gate_url` - araneaDeviceGate API URL (例: https://api.example.com/api/araneaDeviceGate)
    /// * `pool` - MySQL connection pool
    /// * `config_store` - ConfigStore instance
    pub fn new(gate_url: String, pool: MySqlPool, config_store: Arc<ConfigStore>) -> Self {
        Self {
            gate_url,
            http_client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to create HTTP client"),
            repository: AraneaRegistrationRepository::new(pool),
            config_store,
        }
    }

    /// Register device to araneaDeviceGate
    ///
    /// ## 処理フロー
    /// 1. 既存登録チェック（登録済みなら既存情報返却）
    /// 2. MACアドレス取得
    /// 3. LacisID生成
    /// 4. araneaDeviceGate API呼び出し
    /// 5. 成功時: config_store + DB永続化
    ///
    /// ## エラー
    /// - HTTP 409: MAC重複（既登録）
    /// - HTTP 401/403: 認証失敗
    /// - HTTP 5xx: サーバーエラー
    pub async fn register_device(
        &self,
        request: RegisterRequest,
    ) -> crate::Result<RegisterResult> {
        // 1. 既存登録チェック
        let status = self.get_registration_status().await?;
        if status.registered {
            tracing::info!(
                lacis_id = ?status.lacis_id,
                "AraneaRegister: Already registered, returning existing info"
            );
            return Ok(RegisterResult {
                ok: true,
                lacis_id: status.lacis_id,
                cic: status.cic,
                state_endpoint: self.get_config_value(config_keys::STATE_ENDPOINT).await,
                mqtt_endpoint: self.get_config_value(config_keys::MQTT_ENDPOINT).await,
                error: None,
            });
        }

        // 2. MACアドレス取得
        let mac = get_primary_mac_address().map_err(|e| {
            tracing::error!(error = %e, "AraneaRegister: Failed to get MAC address");
            crate::Error::Internal(format!("Failed to get MAC address: {}", e))
        })?;

        tracing::info!(mac = %mac, "AraneaRegister: Got MAC address");

        // 3. LacisID生成
        let lacis_id = try_generate_lacis_id(&mac).map_err(|e| {
            tracing::error!(error = %e, "AraneaRegister: Failed to generate LacisID");
            crate::Error::Validation(e)
        })?;

        tracing::info!(lacis_id = %lacis_id, "AraneaRegister: Generated LacisID");

        // 4. araneaDeviceGate API呼び出し
        let result = self
            .call_device_gate(&request.tenant_primary_auth, &request.tid, &lacis_id, &mac)
            .await?;

        // 5. 成功時: 永続化
        if result.ok {
            self.save_registration(&result, &request.tid, request.fid.as_deref()).await?;
            tracing::info!(
                lacis_id = %lacis_id,
                cic = ?result.cic,
                fid = ?request.fid,
                "AraneaRegister: Registration successful"
            );
        } else {
            tracing::warn!(
                error = ?result.error,
                "AraneaRegister: Registration failed"
            );
        }

        Ok(result)
    }

    /// Call araneaDeviceGate API
    async fn call_device_gate(
        &self,
        auth: &TenantPrimaryAuth,
        tid: &str,
        lacis_id: &str,
        mac: &str,
    ) -> crate::Result<RegisterResult> {
        // ESP32準拠: gateUrlにそのままPOST（/registerは付けない）
        let url = self.gate_url.clone();

        // MACアドレスからコロンを除去（API要求: 12桁hex）
        let mac_hex = mac.replace(":", "");

        let payload = json!({
            "lacisOath": {
                "lacisId": auth.lacis_id,
                "userId": auth.user_id,
                "cic": auth.cic,
                "method": "register"
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": tid,
                "typeDomain": TYPE_DOMAIN,
                "type": DEVICE_TYPE
            },
            "deviceMeta": {
                "macAddress": mac_hex,
                "productType": PRODUCT_TYPE,
                "productCode": PRODUCT_CODE
            }
        });

        tracing::debug!(
            url = %url,
            payload = %payload,
            "AraneaRegister: Calling araneaDeviceGate"
        );

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "AraneaRegister: HTTP request failed");
                crate::Error::Network(format!("HTTP request failed: {}", e))
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!(error = %e, "AraneaRegister: Failed to parse response");
            crate::Error::Internal(format!("Failed to parse response: {}", e))
        })?;

        tracing::debug!(
            status = %status,
            body = %body,
            "AraneaRegister: Got response"
        );

        match status.as_u16() {
            200 | 201 => {
                let ok = body["ok"].as_bool().unwrap_or(false);
                if ok {
                    Ok(RegisterResult {
                        ok: true,
                        lacis_id: Some(lacis_id.to_string()),
                        cic: body["userObject"]["cic_code"]
                            .as_str()
                            .map(String::from),
                        state_endpoint: body["stateEndpoint"].as_str().map(String::from),
                        mqtt_endpoint: body["mqttEndpoint"].as_str().map(String::from),
                        error: None,
                    })
                } else {
                    Ok(RegisterResult {
                        ok: false,
                        lacis_id: None,
                        cic: None,
                        state_endpoint: None,
                        mqtt_endpoint: None,
                        error: body["error"].as_str().map(String::from),
                    })
                }
            }
            409 => {
                tracing::warn!("AraneaRegister: Device already registered (409 Conflict)");
                Ok(RegisterResult {
                    ok: false,
                    lacis_id: Some(lacis_id.to_string()),
                    cic: None,
                    state_endpoint: None,
                    mqtt_endpoint: None,
                    error: Some("Device already registered (conflict)".to_string()),
                })
            }
            401 | 403 => {
                tracing::error!(
                    status = %status,
                    "AraneaRegister: Authentication failed"
                );
                Ok(RegisterResult {
                    ok: false,
                    lacis_id: None,
                    cic: None,
                    state_endpoint: None,
                    mqtt_endpoint: None,
                    error: Some(format!(
                        "Authentication failed ({}): {}",
                        status,
                        body["error"].as_str().unwrap_or("Unknown error")
                    )),
                })
            }
            _ => {
                tracing::error!(
                    status = %status,
                    body = %body,
                    "AraneaRegister: Unexpected response"
                );
                Ok(RegisterResult {
                    ok: false,
                    lacis_id: None,
                    cic: None,
                    state_endpoint: None,
                    mqtt_endpoint: None,
                    error: body["error"]
                        .as_str()
                        .map(String::from)
                        .or_else(|| Some(format!("Unexpected status: {}", status))),
                })
            }
        }
    }

    /// Save registration to config_store and DB
    async fn save_registration(
        &self,
        result: &RegisterResult,
        tid: &str,
        fid: Option<&str>,
    ) -> crate::Result<()> {
        // config_storeに保存 (即時参照用)
        if let Some(ref lacis_id) = result.lacis_id {
            self.set_config_value(config_keys::LACIS_ID, lacis_id)
                .await?;
        }
        if let Some(ref cic) = result.cic {
            self.set_config_value(config_keys::CIC, cic).await?;
        }
        if let Some(ref endpoint) = result.state_endpoint {
            self.set_config_value(config_keys::STATE_ENDPOINT, endpoint)
                .await?;
        }
        if let Some(ref endpoint) = result.mqtt_endpoint {
            self.set_config_value(config_keys::MQTT_ENDPOINT, endpoint)
                .await?;
        }
        self.set_config_value(config_keys::TID, tid).await?;
        if let Some(fid_value) = fid {
            self.set_config_value(config_keys::FID, fid_value).await?;
        }
        self.set_config_value(config_keys::REGISTERED, "true")
            .await?;

        // DBに保存 (履歴・監査用)
        let registration = AraneaRegistration {
            id: None,
            lacis_id: result.lacis_id.clone().unwrap_or_default(),
            tid: tid.to_string(),
            fid: fid.map(String::from),
            cic: result.cic.clone().unwrap_or_default(),
            device_type: DEVICE_TYPE.to_string(),
            state_endpoint: result.state_endpoint.clone(),
            mqtt_endpoint: result.mqtt_endpoint.clone(),
            registered_at: Utc::now(),
            last_sync_at: None,
        };

        self.repository.insert(&registration).await?;

        tracing::info!(
            lacis_id = %registration.lacis_id,
            fid = ?fid,
            "AraneaRegister: Registration saved to config_store and DB"
        );

        Ok(())
    }

    /// Get registration status
    pub async fn get_registration_status(&self) -> crate::Result<RegistrationStatus> {
        let registered = self
            .get_config_value(config_keys::REGISTERED)
            .await
            .map(|v| v == "true")
            .unwrap_or(false);

        if !registered {
            return Ok(RegistrationStatus::default());
        }

        let lacis_id = self.get_config_value(config_keys::LACIS_ID).await;
        let tid = self.get_config_value(config_keys::TID).await;
        let fid = self.get_config_value(config_keys::FID).await;
        let cic = self.get_config_value(config_keys::CIC).await;

        // DB から timestamp取得
        let registered_at = self.repository.get_registered_at().await.ok().flatten();
        let last_sync_at = self.repository.get_last_sync_at().await.ok().flatten();

        Ok(RegistrationStatus {
            registered: true,
            lacis_id,
            tid,
            fid,
            cic,
            registered_at,
            last_sync_at,
        })
    }

    /// Clear registration
    pub async fn clear_registration(&self) -> crate::Result<ClearResult> {
        // config_storeからクリア
        self.remove_config_value(config_keys::LACIS_ID).await?;
        self.remove_config_value(config_keys::TID).await?;
        self.remove_config_value(config_keys::FID).await?;
        self.remove_config_value(config_keys::CIC).await?;
        self.remove_config_value(config_keys::STATE_ENDPOINT).await?;
        self.remove_config_value(config_keys::MQTT_ENDPOINT).await?;
        self.remove_config_value(config_keys::REGISTERED).await?;

        // DBからクリア
        let count = self.repository.delete_all().await?;

        tracing::info!(
            count = count,
            "AraneaRegister: Registration cleared"
        );

        Ok(ClearResult {
            ok: true,
            message: format!("Registration cleared ({} records deleted)", count),
        })
    }

    /// Update last_sync_at timestamp
    pub async fn update_last_sync(&self) -> crate::Result<()> {
        let lacis_id = self
            .get_config_value(config_keys::LACIS_ID)
            .await
            .ok_or_else(|| crate::Error::NotFound("Not registered".to_string()))?;

        self.repository.update_last_sync_at(&lacis_id).await
    }

    /// Get LacisID (if registered)
    pub async fn get_lacis_id(&self) -> Option<String> {
        self.get_config_value(config_keys::LACIS_ID).await
    }

    /// Get CIC (if registered)
    pub async fn get_cic(&self) -> Option<String> {
        self.get_config_value(config_keys::CIC).await
    }

    /// Get TID (if registered)
    pub async fn get_tid(&self) -> Option<String> {
        self.get_config_value(config_keys::TID).await
    }

    /// Get FID (if set)
    pub async fn get_fid(&self) -> Option<String> {
        self.get_config_value(config_keys::FID).await
    }

    /// Set FID
    pub async fn set_fid(&self, fid: &str) -> crate::Result<()> {
        self.set_config_value(config_keys::FID, fid).await
    }

    /// Check if registered
    pub async fn is_registered(&self) -> bool {
        self.get_config_value(config_keys::REGISTERED)
            .await
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    // ========================================
    // config_store helpers
    // ========================================

    async fn get_config_value(&self, key: &str) -> Option<String> {
        self.config_store
            .service()
            .get_setting(key)
            .await
            .ok()
            .flatten()
            .and_then(|v| v.as_str().map(String::from))
    }

    async fn set_config_value(&self, key: &str, value: &str) -> crate::Result<()> {
        self.config_store
            .service()
            .set_setting(key, serde_json::json!(value))
            .await
    }

    async fn remove_config_value(&self, key: &str) -> crate::Result<()> {
        // config_storeにremoveがない場合は空文字列で上書き
        // または専用のdelete_settingメソッドがあれば使用
        self.config_store
            .service()
            .set_setting(key, serde_json::json!(null))
            .await
    }
}
