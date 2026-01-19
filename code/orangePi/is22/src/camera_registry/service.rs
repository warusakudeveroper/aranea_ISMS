//! CameraRegistry Service
//!
//! Phase 2: CameraRegistry (Issue #115)
//! DD05_CameraRegistry.md準拠
//!
//! ## 概要
//! カメラをis801 paraclateCamera仮想デバイスとして登録し、
//! araneaDeviceGateからCICを取得する。
//!
//! ## 処理フロー
//! 1. MACアドレスからLacisID生成
//! 2. araneaDeviceGateへ登録リクエスト
//! 3. CIC取得・DB保存
//! 4. camerasテーブル更新

use super::lacis_id::try_generate_camera_lacis_id;
use super::repository::CameraRegistrationRepository;
use super::types::{
    CameraRegisterRequest, CameraRegisterResponse, CameraRegistration, CameraRegistrationStatus,
    DeviceMeta, GateRegisterPayload, GateRegisterResponse, LacisOath, RegistrationState,
    UserObject, DEVICE_TYPE, PRODUCT_TYPE,
};
use crate::config_store::ConfigStore;
use chrono::Utc;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// CameraRegistry Service
pub struct CameraRegistryService {
    /// araneaDeviceGate URL
    gate_url: String,
    /// DB Repository
    repository: CameraRegistrationRepository,
    /// Config Store (TID, CIC取得用)
    config_store: Arc<ConfigStore>,
    /// HTTP Client
    client: reqwest::Client,
}

impl CameraRegistryService {
    /// 新しいCameraRegistryServiceを作成
    pub fn new(
        gate_url: impl Into<String>,
        repository: CameraRegistrationRepository,
        config_store: Arc<ConfigStore>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            gate_url: gate_url.into(),
            repository,
            config_store,
            client,
        }
    }

    /// カメラを登録
    ///
    /// # Arguments
    /// * `request` - 登録リクエスト
    ///
    /// # Returns
    /// 登録結果（LacisID, CIC）
    pub async fn register_camera(
        &self,
        request: CameraRegisterRequest,
    ) -> crate::Result<CameraRegisterResponse> {
        info!(
            camera_id = %request.camera_id,
            mac = %request.mac_address,
            "Starting camera registration"
        );

        // 0. MACアドレス正規化 (コロン/ハイフン除去、大文字12桁)
        let normalized_mac = request
            .mac_address
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>()
            .to_uppercase();

        if normalized_mac.len() != 12 {
            return Ok(CameraRegisterResponse::error(format!(
                "Invalid MAC address: {} (expected 12 hex digits, got {})",
                request.mac_address,
                normalized_mac.len()
            )));
        }

        debug!(
            original_mac = %request.mac_address,
            normalized_mac = %normalized_mac,
            "Normalized MAC address"
        );

        // 1. LacisID生成
        let lacis_id = match try_generate_camera_lacis_id(
            &normalized_mac,
            Some(&request.product_code),
        ) {
            Ok(id) => id,
            Err(e) => {
                error!(error = %e, "Failed to generate LacisID");
                return Ok(CameraRegisterResponse::error(format!(
                    "Invalid MAC address: {}",
                    e
                )));
            }
        };

        debug!(lacis_id = %lacis_id, "Generated LacisID");

        // 2. ConfigStoreからTID, テナントプライマリ認証情報を取得
        // AUTH_SPEC.md Section 5.2: 登録実行者はTenant Primary (permission >= 61)
        let tid = self
            .get_config_value("aranea.tid")
            .await
            .ok_or_else(|| crate::Error::Config("TID not configured".into()))?;

        let tenant_primary_lacis_id = self
            .get_config_value("tenant_primary.lacis_id")
            .await
            .ok_or_else(|| crate::Error::Config("Tenant Primary LacisID not configured".into()))?;

        let tenant_primary_email = self
            .get_config_value("tenant_primary.email")
            .await
            .ok_or_else(|| crate::Error::Config("Tenant Primary Email not configured".into()))?;

        let tenant_primary_cic = self
            .get_config_value("tenant_primary.cic")
            .await
            .ok_or_else(|| crate::Error::Config("Tenant Primary CIC not configured".into()))?;

        // 3. araneaDeviceGateへ登録
        let gate_response = self
            .register_to_gate(
                &lacis_id,
                &tid,
                &tenant_primary_lacis_id,
                &tenant_primary_email,
                &tenant_primary_cic,
                &normalized_mac,
                &request,
            )
            .await;

        let (cic, state, error_message) = match gate_response {
            Ok(resp) => {
                // CICはuserObject.cic_codeから取得
                if let Some(cic) = resp.get_cic() {
                    info!(
                        lacis_id = %lacis_id,
                        cic = %cic,
                        existing = resp.existing.unwrap_or(false),
                        "Camera registered to araneaDeviceGate"
                    );
                    (cic, RegistrationState::Registered, None)
                } else {
                    let err = resp.error.clone().unwrap_or_else(|| "No CIC in response".to_string());
                    warn!(
                        lacis_id = %lacis_id,
                        error = %err,
                        ok = ?resp.ok,
                        existing = ?resp.existing,
                        user_object = ?resp.user_object,
                        "Gate registration failed - no CIC received"
                    );
                    (String::new(), RegistrationState::Failed, Some(err))
                }
            }
            Err(e) => {
                error!(error = %e, "Gate request failed");
                (
                    String::new(),
                    RegistrationState::Failed,
                    Some(e.to_string()),
                )
            }
        };

        // 4. DB保存
        let registration = CameraRegistration {
            camera_id: request.camera_id.clone(),
            lacis_id: lacis_id.clone(),
            tid: tid.clone(),
            fid: request.fid.clone(),
            rid: request.rid.clone(),
            cic: cic.clone(),
            state: state.clone(),
            registered_at: if state == RegistrationState::Registered {
                Some(Utc::now())
            } else {
                None
            },
            error_message: error_message.clone(),
        };

        if let Err(e) = self.repository.save_registration(&registration).await {
            error!(error = %e, "Failed to save registration");
            return Err(e);
        }

        // 5. レスポンス生成
        if state == RegistrationState::Registered {
            Ok(CameraRegisterResponse::success(lacis_id, cic))
        } else {
            Ok(CameraRegisterResponse::error(
                error_message.unwrap_or_else(|| "Registration failed".to_string()),
            ))
        }
    }

    /// araneaDeviceGateへ登録リクエスト
    /// AUTH_SPEC.md Section 5.3準拠: lacisOathはテナントプライマリの認証情報
    async fn register_to_gate(
        &self,
        camera_lacis_id: &str,
        tid: &str,
        tenant_primary_lacis_id: &str,
        tenant_primary_email: &str,
        tenant_primary_cic: &str,
        normalized_mac: &str,
        request: &CameraRegisterRequest,
    ) -> crate::Result<GateRegisterResponse> {
        // ペイロード構築 (AUTH_SPEC.md Section 5.3準拠)
        let payload = GateRegisterPayload {
            lacis_oath: LacisOath {
                // lacisOath: テナントプライマリの認証情報
                lacis_id: tenant_primary_lacis_id.to_string(),
                user_id: tenant_primary_email.to_string(),
                cic: tenant_primary_cic.to_string(),
                method: "register".to_string(),
            },
            user_object: UserObject {
                // userObject: 登録するカメラの情報
                lacis_id: camera_lacis_id.to_string(),
                tid: tid.to_string(),
                type_domain: "araneaDevice".to_string(),
                device_type: DEVICE_TYPE.to_string(),
            },
            device_meta: DeviceMeta {
                mac_address: normalized_mac.to_string(),
                product_type: PRODUCT_TYPE.to_string(),
                product_code: request.product_code.clone(),
                parent_lacis_id: self.get_config_value("aranea.lacis_id").await, // IS22のLacisID
                camera_name: None,
            },
        };

        debug!(
            gate_url = %self.gate_url,
            "Sending registration request to Gate"
        );

        let response = self
            .client
            .post(&self.gate_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        debug!(
            status = %status,
            body = %body,
            "Gate response received"
        );

        if !status.is_success() {
            error!(
                status = %status,
                body = %body,
                "Gate returned error status"
            );
            return Err(crate::Error::Api(format!(
                "Gate returned {}: {}",
                status, body
            )));
        }

        serde_json::from_str::<GateRegisterResponse>(&body)
            .map_err(|e| {
                error!(error = %e, body = %body, "Failed to parse Gate response");
                crate::Error::Parse(format!("{}: {}", e, body))
            })
    }

    /// カメラの登録状態を取得
    pub async fn get_registration(
        &self,
        camera_id: &str,
    ) -> crate::Result<Option<CameraRegistrationStatus>> {
        let reg = self.repository.get_registration(camera_id).await?;
        Ok(reg.map(|r| r.into()))
    }

    /// LacisIDでカメラを検索
    pub async fn get_camera_by_lacis_id(
        &self,
        lacis_id: &str,
    ) -> crate::Result<Option<CameraRegistrationStatus>> {
        let reg = self.repository.get_by_lacis_id(lacis_id).await?;
        Ok(reg.map(|r| r.into()))
    }

    /// テナント配下の登録済みカメラ一覧を取得
    pub async fn get_registered_cameras(
        &self,
        tid: &str,
    ) -> crate::Result<Vec<CameraRegistrationStatus>> {
        let regs = self.repository.get_registered_cameras(tid).await?;
        Ok(regs.into_iter().map(|r| r.into()).collect())
    }

    /// カメラの登録をクリア
    pub async fn clear_registration(&self, camera_id: &str) -> crate::Result<()> {
        info!(camera_id = %camera_id, "Clearing camera registration");
        self.repository.clear_registration(camera_id).await
    }

    /// カメラを再アクティベート（登録クリア→再登録）
    ///
    /// # Arguments
    /// * `camera_id` - カメラID
    ///
    /// # Returns
    /// 新しいCIC付きの登録結果
    pub async fn reactivate_camera(
        &self,
        camera_id: &str,
    ) -> crate::Result<CameraRegisterResponse> {
        info!(camera_id = %camera_id, "Reactivating camera (clear + re-register)");

        // 1. 現在の登録をクリア
        self.clear_registration(camera_id).await?;

        // 2. カメラ情報を取得
        let camera_info = self.get_camera_for_registration(camera_id).await?;
        let Some(info) = camera_info else {
            return Ok(CameraRegisterResponse::error(format!(
                "Camera {} not found or has no MAC address",
                camera_id
            )));
        };

        // 3. 再登録
        let request = CameraRegisterRequest {
            camera_id: camera_id.to_string(),
            mac_address: info.mac_address,
            product_code: info.product_code.unwrap_or_else(|| "0000".to_string()),
            fid: info.fid,
            rid: info.rid,
        };

        let result = self.register_camera(request).await?;

        if result.ok {
            info!(
                camera_id = %camera_id,
                new_cic = %result.cic.as_deref().unwrap_or(""),
                "Camera reactivated successfully"
            );
        } else {
            warn!(
                camera_id = %camera_id,
                error = %result.error.as_deref().unwrap_or("unknown"),
                "Camera reactivation failed"
            );
        }

        Ok(result)
    }

    /// 登録用カメラ情報を取得
    async fn get_camera_for_registration(
        &self,
        camera_id: &str,
    ) -> crate::Result<Option<CameraRegistrationInfo>> {
        let row = sqlx::query_as::<_, CameraRegistrationInfoRow>(
            r#"
            SELECT
                c.camera_id,
                c.mac_address,
                cb.product_code,
                c.fid,
                c.rid
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.camera_id = ? AND c.deleted_at IS NULL
            "#,
        )
        .bind(camera_id)
        .fetch_optional(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(row.and_then(|r| {
            r.mac_address.map(|mac| CameraRegistrationInfo {
                camera_id: r.camera_id,
                mac_address: mac,
                product_code: r.product_code,
                fid: r.fid,
                rid: r.rid,
            })
        }))
    }

    /// カメラのMACアドレスからLacisIDを生成（登録せずに確認用）
    pub async fn preview_lacis_id(
        &self,
        camera_id: &str,
    ) -> crate::Result<Option<String>> {
        // DBからMACアドレスを取得
        let mac = self.repository.get_camera_mac(camera_id).await?;
        let product_code = self
            .repository
            .get_camera_product_code(camera_id)
            .await?
            .unwrap_or_else(|| "0000".to_string());

        match mac {
            Some(m) => {
                let lacis_id = try_generate_camera_lacis_id(&m, Some(&product_code))
                    .map_err(|e| crate::Error::Validation(e))?;
                Ok(Some(lacis_id))
            }
            None => Ok(None),
        }
    }

    /// 未登録カメラを一括登録
    pub async fn register_pending_cameras(&self) -> crate::Result<Vec<CameraRegisterResponse>> {
        let tid = self
            .get_config_value("aranea.tid")
            .await
            .ok_or_else(|| crate::Error::Config("TID not configured".into()))?;

        // pending状態のカメラを取得
        let pending_cameras = self.get_pending_cameras(&tid).await?;

        let mut results = Vec::new();
        for camera in pending_cameras {
            let request = CameraRegisterRequest {
                camera_id: camera.camera_id,
                mac_address: camera.mac_address,
                product_code: camera.product_code.unwrap_or_else(|| "0000".to_string()),
                fid: camera.fid,
                rid: camera.rid,
            };

            let result = self.register_camera(request).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// pending状態のカメラ一覧を取得
    async fn get_pending_cameras(&self, tid: &str) -> crate::Result<Vec<PendingCamera>> {
        let rows = sqlx::query_as::<_, PendingCameraRow>(
            r#"
            SELECT
                c.camera_id,
                c.mac_address,
                cb.product_code,
                c.fid,
                c.rid
            FROM cameras c
            LEFT JOIN oui_entries oe ON UPPER(SUBSTRING(c.mac_address, 1, 8)) = oe.oui_prefix
            LEFT JOIN camera_brands cb ON oe.brand_id = cb.id
            WHERE c.tid = ? AND (c.registration_state = 'pending' OR c.registration_state IS NULL)
            AND c.mac_address IS NOT NULL
            "#,
        )
        .bind(tid)
        .fetch_all(self.repository.pool())
        .await
        .map_err(|e| crate::Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // ========================================
    // config_store helpers
    // ========================================

    /// ConfigStoreから設定値を取得
    async fn get_config_value(&self, key: &str) -> Option<String> {
        self.config_store
            .service()
            .get_setting(key)
            .await
            .ok()
            .flatten()
            .and_then(|v| v.as_str().map(String::from))
    }
}

/// pending状態のカメラ情報
#[derive(Debug)]
struct PendingCamera {
    camera_id: String,
    mac_address: String,
    product_code: Option<String>,
    fid: Option<String>,
    rid: Option<String>,
}

/// 登録用カメラ情報
#[derive(Debug)]
struct CameraRegistrationInfo {
    camera_id: String,
    mac_address: String,
    product_code: Option<String>,
    fid: Option<String>,
    rid: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CameraRegistrationInfoRow {
    camera_id: String,
    mac_address: Option<String>,
    product_code: Option<String>,
    fid: Option<String>,
    rid: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PendingCameraRow {
    camera_id: String,
    mac_address: String,
    product_code: Option<String>,
    fid: Option<String>,
    rid: Option<String>,
}

impl From<PendingCameraRow> for PendingCamera {
    fn from(row: PendingCameraRow) -> Self {
        Self {
            camera_id: row.camera_id,
            mac_address: row.mac_address,
            product_code: row.product_code,
            fid: row.fid,
            rid: row.rid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_constant() {
        // TYPE_REGISTRY.md準拠: ar-is801ParaclateCamera
        assert_eq!(DEVICE_TYPE, "ar-is801ParaclateCamera");
    }

    #[test]
    fn test_product_type_constant() {
        assert_eq!(PRODUCT_TYPE, "801");
    }
}
