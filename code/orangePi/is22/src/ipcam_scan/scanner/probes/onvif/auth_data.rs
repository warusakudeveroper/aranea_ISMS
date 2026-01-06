use std::net::IpAddr;
use std::time::Duration;

use super::auth_base::{generate_ws_security_header, probe_onvif_mac, probe_onvif_with_auth};
use super::types::{
    OnvifCapabilities,
    OnvifExtendedInfo,
    OnvifNetworkInterface,
    OnvifScopes,
};
use super::xml::{
    extract_capability_xaddr,
    extract_xml_attribute,
    extract_xml_value,
};

/// Probe ONVIF GetScopes with authentication
pub async fn probe_onvif_scopes(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<OnvifScopes> {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let security_header = generate_ws_security_header(username, password);

    let soap_body = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetScopes xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>
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
                    if body.contains("NotAuthorized") {
                        continue;
                    }

                    let mut scopes = OnvifScopes::default();

                    // Extract all ScopeDef/ScopeItem values
                    let scope_patterns = ["ScopeDef", "ScopeItem"];
                    for pattern in scope_patterns {
                        let mut search_pos = 0;
                        let search_pattern = format!(":{}>", pattern);
                        while let Some(start) = body[search_pos..].find(search_pattern.as_str()) {
                            let abs_start = search_pos + start + pattern.len() + 2;
                            if let Some(end) = body[abs_start..].find("</") {
                                let scope_uri = body[abs_start..abs_start + end].trim();
                                if !scope_uri.is_empty() {
                                    scopes.raw_scopes.push(scope_uri.to_string());

                                    // Parse scope URI to extract specific fields
                                    if scope_uri.contains("/name/") {
                                        if let Some(name) = scope_uri.split("/name/").last() {
                                            scopes.name = Some(name.to_string());
                                        }
                                    }
                                    if scope_uri.contains("/hardware/") {
                                        if let Some(hw) = scope_uri.split("/hardware/").last() {
                                            scopes.hardware = Some(hw.to_string());
                                        }
                                    }
                                    if scope_uri.contains("/Profile/") {
                                        if let Some(prof) = scope_uri.split("/Profile/").last() {
                                            scopes.profile = Some(prof.to_string());
                                        }
                                    }
                                    if scope_uri.contains("/location/") {
                                        if let Some(loc) = scope_uri.split("/location/").last() {
                                            scopes.location = Some(loc.to_string());
                                        }
                                    }
                                    if scope_uri.contains("/type/") {
                                        if let Some(dt) = scope_uri.split("/type/").last() {
                                            scopes.device_type = Some(dt.to_string());
                                        }
                                    }
                                }
                                search_pos = abs_start + end;
                            } else {
                                break;
                            }
                        }
                    }

                    if !scopes.raw_scopes.is_empty() {
                        return Some(scopes);
                    }
                }
            }
        }
    }

    None
}

/// Probe ONVIF GetNetworkInterfaces with full data
pub async fn probe_onvif_network_interfaces_full(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Vec<OnvifNetworkInterface> {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let security_header = generate_ws_security_header(username, password);

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
                    if body.contains("NotAuthorized") {
                        continue;
                    }

                    let mut interfaces = Vec::new();
                    let mut iface = OnvifNetworkInterface::default();

                    // Extract NetworkInterface data
                    iface.token = extract_xml_attribute(&body, "NetworkInterface", "token");
                    iface.hw_address = extract_xml_value(&body, "HwAddress");
                    iface.enabled = extract_xml_value(&body, "Enabled")
                        .map(|v| v.to_lowercase() == "true");
                    iface.mtu = extract_xml_value(&body, "MTU")
                        .and_then(|v| v.parse().ok());

                    // IPv4 configuration
                    if body.contains("IPv4") {
                        iface.ipv4_enabled = extract_xml_value(&body, "Enabled")
                            .map(|v| v.to_lowercase() == "true");
                        iface.ipv4_address = extract_xml_value(&body, "Address");
                        iface.ipv4_prefix = extract_xml_value(&body, "PrefixLength")
                            .and_then(|v| v.parse().ok());
                        iface.dhcp = extract_xml_value(&body, "DHCP")
                            .map(|v| v.to_lowercase() == "true");
                    }

                    if iface.hw_address.is_some() || iface.token.is_some() {
                        interfaces.push(iface);
                    }

                    if !interfaces.is_empty() {
                        return interfaces;
                    }
                }
            }
        }
    }

    Vec::new()
}

/// Probe ONVIF GetCapabilities with authentication
pub async fn probe_onvif_capabilities(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<OnvifCapabilities> {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let security_header = generate_ws_security_header(username, password);

    let soap_body = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetCapabilities xmlns=\"http://www.onvif.org/ver10/device/wsdl\">
      <Category>All</Category>
    </GetCapabilities>
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
                    if body.contains("NotAuthorized") {
                        continue;
                    }

                    let mut caps = OnvifCapabilities::default();

                    // Analytics
                    caps.analytics_support = body.contains("<Analytics>") || body.contains(":Analytics>");
                    if caps.analytics_support {
                        caps.analytics_xaddr = extract_capability_xaddr(&body, "Analytics");
                    }

                    // Device
                    caps.device_xaddr = extract_capability_xaddr(&body, "Device");
                    caps.device_io_support = body.contains("<IO>") || body.contains(":IO>");

                    // Events
                    caps.events_support = body.contains("<Events>") || body.contains(":Events>");
                    if caps.events_support {
                        caps.events_xaddr = extract_capability_xaddr(&body, "Events");
                        caps.ws_subscription_support = body.contains("WSSubscriptionPolicySupport")
                            && body.contains("true");
                        caps.ws_pullpoint_support = body.contains("WSPullPointSupport")
                            && body.contains("true");
                    }

                    // Imaging
                    caps.imaging_support = body.contains("<Imaging>") || body.contains(":Imaging>");
                    if caps.imaging_support {
                        caps.imaging_xaddr = extract_capability_xaddr(&body, "Imaging");
                    }

                    // Media
                    caps.media_support = body.contains("<Media>") || body.contains(":Media>");
                    if caps.media_support {
                        caps.media_xaddr = extract_capability_xaddr(&body, "Media");
                        caps.rtsp_streaming = body.contains("RTPMulticast") || body.contains("RTSP");
                    }

                    // PTZ
                    caps.ptz_support = body.contains("<PTZ>") || body.contains(":PTZ>");
                    if caps.ptz_support {
                        caps.ptz_xaddr = extract_capability_xaddr(&body, "PTZ");
                    }

                    // Return if we got any capabilities
                    if caps.device_xaddr.is_some() || caps.media_support || caps.events_support {
                        return Some(caps);
                    }
                }
            }
        }
    }

    None
}

/// Probe ONVIF with authentication and get extended info
pub async fn probe_onvif_extended(
    ip: IpAddr,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<OnvifExtendedInfo> {
    // First get basic device info
    let device_info = probe_onvif_with_auth(ip, username, password, timeout_ms).await?;

    // Collect extended data in parallel
    let scopes_fut = probe_onvif_scopes(ip, username, password, timeout_ms);
    let network_fut = probe_onvif_network_interfaces_full(ip, username, password, timeout_ms);
    let caps_fut = probe_onvif_capabilities(ip, username, password, timeout_ms);

    let (scopes, network_interfaces, capabilities) = tokio::join!(scopes_fut, network_fut, caps_fut);

    Some(OnvifExtendedInfo {
        device_info,
        scopes,
        network_interfaces,
        capabilities,
    })
}
