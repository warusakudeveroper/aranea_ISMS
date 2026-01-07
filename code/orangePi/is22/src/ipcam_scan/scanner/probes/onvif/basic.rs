use std::net::IpAddr;
use std::time::Duration;

use crate::ipcam_scan::scanner::probes::ProbeResult;

/// Stage 4: ONVIF probe (simplified - GetSystemDateAndTime)
/// Returns (success_bool, detailed_result)
pub async fn probe_onvif(ip: IpAddr, timeout_ms: u32) -> bool {
    probe_onvif_detailed(ip, timeout_ms).await.0
}

/// Stage 4: ONVIF probe with detailed result
pub async fn probe_onvif_detailed(ip: IpAddr, timeout_ms: u32) -> (bool, ProbeResult) {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    // ONVIF GetSystemDateAndTime - simplest unauthenticated call
    let soap_body = r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
<s:Body><GetSystemDateAndTime xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/></s:Body>
</s:Envelope>"#;

    let mut last_result = ProbeResult::NotTested;

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
            .body(soap_body.to_string())
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    if let Ok(body) = resp.text().await {
                        if body.contains("SystemDateAndTime") || body.contains("UTCDateTime") {
                            return (true, ProbeResult::Success);
                        }
                        // Got response but not ONVIF
                        last_result = ProbeResult::NoResponse;
                    }
                } else if status == reqwest::StatusCode::UNAUTHORIZED {
                    last_result = ProbeResult::AuthRequired;
                } else {
                    last_result = ProbeResult::NoResponse;
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    last_result = ProbeResult::Timeout;
                } else if e.is_connect() {
                    last_result = ProbeResult::Refused;
                } else {
                    last_result = ProbeResult::Error;
                }
            }
        }
    }

    (false, last_result)
}
