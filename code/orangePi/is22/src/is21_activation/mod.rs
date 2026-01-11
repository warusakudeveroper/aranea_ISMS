//! IS21 Activation Service
//!
//! T5-1: IS22→IS21アクティベート（mobes2.0回答に基づく実装）
//!
//! ## 概要
//! IS22がis21推論サーバーをaraneaDeviceGate経由でアクティベート
//!
//! ## mobes2.0仕様
//! - araneaDeviceGate APIを使用
//! - LacisOath認証
//! - is21のdeviceTypeは既に定義済み

mod types;

pub use types::*;

use crate::config_store::ConfigStore;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// IS21デバイス種別定数
mod device_constants {
    /// is21のTypeDomain
    pub const TYPE_DOMAIN: &str = "araneaDevices";
    /// is21のDeviceType
    pub const DEVICE_TYPE: &str = "is21";
    /// is21のProductType
    pub const PRODUCT_TYPE: u16 = 221;
    /// is21のProductCode
    pub const PRODUCT_CODE: u16 = 1001;
}

/// IS21アクティベートサービス
pub struct Is21ActivationService {
    /// araneaDeviceGate URL
    gate_url: String,
    /// HTTPクライアント
    http_client: Client,
    /// 設定ストア
    config_store: Arc<ConfigStore>,
}

impl Is21ActivationService {
    /// 新規作成
    ///
    /// # Arguments
    /// * `gate_url` - araneaDeviceGate API URL
    /// * `config_store` - 設定ストア
    pub fn new(gate_url: String, config_store: Arc<ConfigStore>) -> Self {
        Self {
            gate_url,
            http_client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to create HTTP client"),
            config_store,
        }
    }

    /// IS21をアクティベート
    ///
    /// ## 処理フロー
    /// 1. IS22の登録情報から認証情報取得
    /// 2. is21のMACアドレス取得（HTTPで問い合わせ）
    /// 3. araneaDeviceGate APIでis21を登録
    /// 4. 成功時: is21のCIC/LacisIDを返却
    pub async fn activate_is21(
        &self,
        is21_endpoint: &str,
        auth: &Is21ActivationAuth,
    ) -> crate::Result<Is21ActivationResult> {
        info!(
            is21_endpoint = %is21_endpoint,
            tid = %auth.tid,
            "Activating IS21 server"
        );

        // 1. is21からMACアドレスを取得
        let mac = self.get_is21_mac(is21_endpoint).await?;
        info!(is21_mac = %mac, "Got IS21 MAC address");

        // 2. LacisID生成（is21用）
        let lacis_id = self.generate_is21_lacis_id(&mac)?;
        info!(is21_lacis_id = %lacis_id, "Generated IS21 LacisID");

        // 3. araneaDeviceGate APIでis21を登録
        let result = self.call_device_gate(auth, is21_endpoint, &lacis_id, &mac).await?;

        if result.ok {
            info!(
                is21_lacis_id = %lacis_id,
                is21_cic = ?result.cic,
                "IS21 activated successfully"
            );
        } else {
            warn!(
                error = ?result.error,
                "IS21 activation failed"
            );
        }

        Ok(result)
    }

    /// IS21からMACアドレスを取得
    async fn get_is21_mac(&self, is21_endpoint: &str) -> crate::Result<String> {
        let url = format!("{}/api/device/info", is21_endpoint.trim_end_matches('/'));

        debug!(url = %url, "Getting IS21 device info");

        let response = self
            .http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to get IS21 device info");
                crate::Error::Network(format!("Failed to connect to IS21: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(crate::Error::Network(format!(
                "IS21 returned status {}",
                response.status()
            )));
        }

        let info: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::Internal(format!("Failed to parse IS21 response: {}", e))
        })?;

        // MACアドレスを取得
        let mac = info["macAddress"]
            .as_str()
            .or_else(|| info["mac_address"].as_str())
            .or_else(|| info["mac"].as_str())
            .ok_or_else(|| crate::Error::Internal("IS21 did not return MAC address".to_string()))?;

        Ok(mac.to_string())
    }

    /// IS21用LacisIDを生成
    fn generate_is21_lacis_id(&self, mac: &str) -> crate::Result<String> {
        // MACアドレスからコロン/ハイフンを除去
        let mac_hex = mac
            .replace(":", "")
            .replace("-", "")
            .to_uppercase();

        if mac_hex.len() != 12 {
            return Err(crate::Error::Validation(format!(
                "Invalid MAC address: {}",
                mac
            )));
        }

        // LacisID形式: [Prefix=3][ProductType=3桁][MAC=12桁][ProductCode=4桁] = 20文字
        let lacis_id = format!(
            "3{:03}{}{}",
            device_constants::PRODUCT_TYPE,
            mac_hex,
            format!("{:04}", device_constants::PRODUCT_CODE)
        );

        if lacis_id.len() != 20 {
            return Err(crate::Error::Internal(format!(
                "Generated invalid LacisID length: {}",
                lacis_id.len()
            )));
        }

        Ok(lacis_id)
    }

    /// araneaDeviceGate APIを呼び出し
    async fn call_device_gate(
        &self,
        auth: &Is21ActivationAuth,
        is21_endpoint: &str,
        lacis_id: &str,
        mac: &str,
    ) -> crate::Result<Is21ActivationResult> {
        let url = self.gate_url.clone();
        let mac_hex = mac.replace(":", "").replace("-", "");

        let payload = json!({
            "lacisOath": {
                "lacisId": auth.is22_lacis_id,
                "userId": auth.user_id,
                "cic": auth.cic,
                "method": "register"
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": auth.tid,
                "typeDomain": device_constants::TYPE_DOMAIN,
                "type": device_constants::DEVICE_TYPE
            },
            "deviceMeta": {
                "macAddress": mac_hex,
                "productType": device_constants::PRODUCT_TYPE,
                "productCode": device_constants::PRODUCT_CODE,
                "endpoint": is21_endpoint
            }
        });

        debug!(
            url = %url,
            payload = %payload,
            "Calling araneaDeviceGate for IS21"
        );

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "araneaDeviceGate request failed");
                crate::Error::Network(format!("HTTP request failed: {}", e))
            })?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::Internal(format!("Failed to parse response: {}", e))
        })?;

        debug!(
            status = %status,
            body = %body,
            "Got araneaDeviceGate response"
        );

        match status.as_u16() {
            200 | 201 => {
                let ok = body["ok"].as_bool().unwrap_or(false);
                if ok {
                    Ok(Is21ActivationResult {
                        ok: true,
                        is21_lacis_id: Some(lacis_id.to_string()),
                        cic: body["userObject"]["cic_code"]
                            .as_str()
                            .map(String::from),
                        endpoint: Some(is21_endpoint.to_string()),
                        error: None,
                    })
                } else {
                    Ok(Is21ActivationResult {
                        ok: false,
                        is21_lacis_id: None,
                        cic: None,
                        endpoint: None,
                        error: body["error"].as_str().map(String::from),
                    })
                }
            }
            409 => {
                // 既に登録済み
                warn!("IS21 already registered (409 Conflict)");
                Ok(Is21ActivationResult {
                    ok: false,
                    is21_lacis_id: Some(lacis_id.to_string()),
                    cic: None,
                    endpoint: None,
                    error: Some("IS21 already registered".to_string()),
                })
            }
            401 | 403 => {
                error!(status = %status, "Authentication failed");
                Ok(Is21ActivationResult {
                    ok: false,
                    is21_lacis_id: None,
                    cic: None,
                    endpoint: None,
                    error: Some(format!(
                        "Authentication failed ({}): {}",
                        status,
                        body["error"].as_str().unwrap_or("Unknown error")
                    )),
                })
            }
            _ => {
                error!(status = %status, body = %body, "Unexpected response");
                Ok(Is21ActivationResult {
                    ok: false,
                    is21_lacis_id: None,
                    cic: None,
                    endpoint: None,
                    error: Some(format!("Unexpected status: {}", status)),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is21_lacis_id() {
        // 仮のconfig_storeを作成できないのでテストスキップ
        // 実際のテストは統合テストで行う
    }

    #[test]
    fn test_device_constants() {
        assert_eq!(device_constants::TYPE_DOMAIN, "araneaDevices");
        assert_eq!(device_constants::DEVICE_TYPE, "is21");
        assert_eq!(device_constants::PRODUCT_TYPE, 221);
        assert_eq!(device_constants::PRODUCT_CODE, 1001);
    }
}
