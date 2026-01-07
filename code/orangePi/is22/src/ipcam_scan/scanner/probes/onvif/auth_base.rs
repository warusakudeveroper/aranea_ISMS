use std::net::IpAddr;
use std::time::Duration;

use rand::Rng;
use sha1::{Digest, Sha1};

use super::types::OnvifDeviceInfo;
use super::xml::extract_xml_value;

/// Generate WS-Security UsernameToken Digest header for ONVIF authentication
pub fn generate_ws_security_header(username: &str, password: &str) -> String {
    // 1. Generate nonce (16 random bytes)
    let mut rng = rand::thread_rng();
    let nonce_bytes: [u8; 16] = rng.gen();
    let nonce_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &nonce_bytes,
    );

    // 2. Generate created timestamp (ISO8601)
    let created = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // 3. Calculate PasswordDigest = Base64(SHA1(nonce + created + password))
    let mut hasher = Sha1::new();
    hasher.update(&nonce_bytes);
    hasher.update(created.as_bytes());
    hasher.update(password.as_bytes());
    let digest = hasher.finalize();
    let digest_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &digest,
    );

    // 4. Build SOAP header
    format!(
        r#"<wsse:Security xmlns:wsse=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd\" xmlns:wsu=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd\">
      <wsse:UsernameToken>
        <wsse:Username>{}</wsse:Username>
        <wsse:Password Type=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest\">{}</wsse:Password>
        <wsse:Nonce EncodingType=\"http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary\">{}</wsse:Nonce>
        <wsu:Created>{}</wsu:Created>
      </wsse:UsernameToken>
    </wsse:Security>"#,
        username, digest_b64, nonce_b64, created
    )
}

/// Probe ONVIF GetDeviceInformation with authentication
pub async fn probe_onvif_with_auth(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<OnvifDeviceInfo> {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let security_header = generate_ws_security_header(username, password);

    // ONVIF GetDeviceInformation SOAP request
    let soap_body = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetDeviceInformation xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>
  </s:Body>
</s:Envelope>"#,
        security_header
    );

    for port in ports {
        let url = format!("http://{}:{}/onvif/device_service", ip, port);

        let client = match reqwest::Client::builder()
            .timeout(timeout_dur)
            .build()
        {
            Ok(c) => c,
            Err(_) => continue,
        };

        let result = client
            .post(&url)
            .header("Content-Type", "application/soap+xml")
            .body(soap_body.clone())
            .send()
            .await;

        if let Ok(resp) = result {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    // Check for auth error
                    if body.contains("NotAuthorized") {
                        continue;
                    }

                    // Parse XML response
                    let mut info = OnvifDeviceInfo::default();

                    // Extract Manufacturer
                    info.manufacturer = extract_xml_value(&body, "Manufacturer");

                    // Extract Model
                    info.model = extract_xml_value(&body, "Model");

                    // Extract FirmwareVersion
                    info.firmware_version = extract_xml_value(&body, "FirmwareVersion");

                    // Extract SerialNumber
                    info.serial_number = extract_xml_value(&body, "SerialNumber");

                    // Extract HardwareId
                    info.hardware_id = extract_xml_value(&body, "HardwareId");

                    // Return if we got at least manufacturer or model
                    if info.manufacturer.is_some() || info.model.is_some() {
                        // Now try to get MAC address via GetNetworkInterfaces
                        if let Some(mac) = probe_onvif_mac(ip, username, password, timeout_ms).await {
                            info.mac_address = Some(mac);
                        }
                        return Some(info);
                    }
                }
            }
        }
    }

    None
}

/// Probe ONVIF GetNetworkInterfaces to get MAC address
pub async fn probe_onvif_mac(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<String> {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let security_header = generate_ws_security_header(username, password);

    // ONVIF GetNetworkInterfaces SOAP request
    let soap_body = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetNetworkInterfaces xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>
  </s:Body>
</s:Envelope>"#,
        security_header
    );

    for port in ports {
        let url = format!("http://{}:{}/onvif/device_service", ip, port);

        let client = match reqwest::Client::builder()
            .timeout(timeout_dur)
            .build()
        {
            Ok(c) => c,
            Err(_) => continue,
        };

        let result = client
            .post(&url)
            .header("Content-Type", "application/soap+xml")
            .body(soap_body.clone())
            .send()
            .await;

        if let Ok(resp) = result {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    // Extract HwAddress (MAC address)
                    if let Some(mac) = extract_xml_value(&body, "HwAddress") {
                        return Some(mac);
                    }
                }
            }
        }
    }

    None
}
