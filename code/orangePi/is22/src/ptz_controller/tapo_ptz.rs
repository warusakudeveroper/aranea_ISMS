//! Tapo ONVIF PTZ Client
//!
//! Tapoカメラ専用のONVIF PTZ制御クライアント
//! WS-Security UsernameToken認証を使用

use crate::error::{Error, Result};
use base64::Engine;
use reqwest::Client;
use sha1::{Digest, Sha1};

/// Tapo ONVIF PTZ制御クライアント
pub struct TapoPtzClient {
    /// ONVIFエンドポイント（例: http://192.168.x.x:2020/onvif/device_service）
    endpoint: String,
    /// 認証ユーザー名
    username: String,
    /// 認証パスワード
    password: String,
    /// HTTPクライアント
    client: Client,
}

impl TapoPtzClient {
    /// 新規作成
    pub fn new(endpoint: &str, username: &str, password: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// PTZサービスURLを取得
    fn ptz_service_url(&self) -> String {
        // device_service を ptz_service に置換
        if self.endpoint.contains("/onvif/device_service") {
            self.endpoint.replace("/onvif/device_service", "/onvif/ptz_service")
        } else {
            // フォールバック: エンドポイントの最後のパスを置換
            let base = self.endpoint.trim_end_matches('/');
            if let Some(pos) = base.rfind('/') {
                format!("{}/ptz_service", &base[..pos])
            } else {
                format!("{}/onvif/ptz_service", base)
            }
        }
    }

    /// 連続移動開始
    ///
    /// # Arguments
    /// * `pan` - パン速度 (-1.0 to 1.0, 負=左, 正=右)
    /// * `tilt` - チルト速度 (-1.0 to 1.0, 負=下, 正=上)
    /// * `zoom` - ズーム速度 (-1.0 to 1.0, 負=ワイド, 正=テレ)
    pub async fn continuous_move(&self, pan: f32, tilt: f32, zoom: f32) -> Result<()> {
        let soap_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
  {}
  <s:Body>
    <tptz:ContinuousMove>
      <tptz:ProfileToken>profile_1</tptz:ProfileToken>
      <tptz:Velocity>
        <tt:PanTilt x="{:.2}" y="{:.2}"/>
        <tt:Zoom x="{:.2}"/>
      </tptz:Velocity>
    </tptz:ContinuousMove>
  </s:Body>
</s:Envelope>"#,
            self.security_header(),
            pan,
            tilt,
            zoom
        );

        self.send_soap_request(&soap_body, "ContinuousMove").await
    }

    /// PTZ停止
    pub async fn stop(&self) -> Result<()> {
        let soap_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  {}
  <s:Body>
    <tptz:Stop>
      <tptz:ProfileToken>profile_1</tptz:ProfileToken>
      <tptz:PanTilt>true</tptz:PanTilt>
      <tptz:Zoom>true</tptz:Zoom>
    </tptz:Stop>
  </s:Body>
</s:Envelope>"#,
            self.security_header()
        );

        self.send_soap_request(&soap_body, "Stop").await
    }

    /// ホームポジションに移動
    pub async fn goto_home(&self) -> Result<()> {
        let soap_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
  {}
  <s:Body>
    <tptz:GotoHomePosition>
      <tptz:ProfileToken>profile_1</tptz:ProfileToken>
    </tptz:GotoHomePosition>
  </s:Body>
</s:Envelope>"#,
            self.security_header()
        );

        self.send_soap_request(&soap_body, "GotoHomePosition").await
    }

    /// WS-Security UsernameToken ヘッダー生成
    fn security_header(&self) -> String {
        // Nonce生成（16バイトランダム）
        let nonce: [u8; 16] = rand::random();
        let nonce_base64 = base64::engine::general_purpose::STANDARD.encode(&nonce);

        // Created タイムスタンプ
        let created = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Password Digest = Base64(SHA1(nonce + created + password))
        let mut hasher = Sha1::new();
        hasher.update(&nonce);
        hasher.update(created.as_bytes());
        hasher.update(self.password.as_bytes());
        let digest = hasher.finalize();
        let digest_base64 = base64::engine::general_purpose::STANDARD.encode(&digest);

        format!(
            r#"<s:Header>
    <Security xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
              s:mustUnderstand="true">
      <UsernameToken>
        <Username>{}</Username>
        <Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</Password>
        <Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</Nonce>
        <Created xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">{}</Created>
      </UsernameToken>
    </Security>
  </s:Header>"#,
            self.username, digest_base64, nonce_base64, created
        )
    }

    /// SOAPリクエスト送信
    async fn send_soap_request(&self, body: &str, action: &str) -> Result<()> {
        let ptz_url = self.ptz_service_url();

        tracing::debug!(
            url = %ptz_url,
            action = %action,
            "Sending ONVIF PTZ request"
        );

        let response = self
            .client
            .post(&ptz_url)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| Error::Network(format!("PTZ request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                status = %status,
                body = %body,
                "ONVIF PTZ request failed"
            );
            return Err(Error::Network(format!(
                "ONVIF PTZ {} failed with status {}: {}",
                action, status, body
            )));
        }

        tracing::info!(action = %action, "ONVIF PTZ command executed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptz_service_url() {
        let client = TapoPtzClient::new(
            "http://192.168.1.100:2020/onvif/device_service",
            "admin",
            "password",
        );
        assert_eq!(
            client.ptz_service_url(),
            "http://192.168.1.100:2020/onvif/ptz_service"
        );
    }

    #[test]
    fn test_security_header_generation() {
        let client = TapoPtzClient::new(
            "http://192.168.1.100:2020/onvif/device_service",
            "admin",
            "testpass",
        );
        let header = client.security_header();
        assert!(header.contains("<Username>admin</Username>"));
        assert!(header.contains("PasswordDigest"));
        assert!(header.contains("<Created"));
    }
}
