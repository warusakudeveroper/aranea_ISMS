//! Scanner implementation for IpcamScan
//!
//! Multi-stage evidence-based camera discovery

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use std::process::Stdio;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::process::Command;
use sha1::{Sha1, Digest};
use rand::Rng;

/// Port weights for scoring
pub const PORT_WEIGHTS: &[(u16, u32)] = &[
    (554, 30),   // RTSP
    (2020, 30),  // ONVIF
    (80, 10),    // HTTP
    (443, 10),   // HTTPS
    (8000, 10),  // NVR
    (8080, 5),   // Alt HTTP
    (8443, 5),   // Alt HTTPS
    (8554, 5),   // Alt RTSP
];

/// OUI prefixes for camera vendors
pub const OUI_CAMERA_VENDORS: &[(&str, &str, u32)] = &[
    // TP-Link / Tapo
    ("70:5A:0F", "TP-LINK", 20),
    ("54:AF:97", "TP-LINK", 20),
    ("B0:A7:B9", "TP-LINK", 20),
    ("6C:5A:B0", "TP-LINK", 20),
    ("6C:C8:40", "TP-LINK", 20),
    ("08:A6:F7", "TP-LINK", 20),
    ("78:20:51", "TP-LINK", 20),
    ("BC:07:1D", "TP-LINK", 20),
    ("18:D6:C7", "TP-LINK", 20),
    ("3C:84:6A", "TP-LINK", 20),  // Tapo C500/C310 etc
    ("D8:07:B6", "TP-LINK", 20),  // Tapo
    ("D8:44:89", "TP-LINK", 20),  // Tapo
    ("9C:53:22", "TP-LINK", 20),  // Tapo C310
    // Google / Nest
    ("F4:F5:D8", "Google", 20),
    ("30:FD:38", "Google", 20),
    ("3C:22:FB", "Google", 20),
    // Hikvision
    ("A8:42:A1", "Hikvision", 20),
    ("C0:56:E3", "Hikvision", 20),
    ("44:19:B6", "Hikvision", 20),
    // Dahua
    ("3C:EF:8C", "Dahua", 20),
    ("4C:11:BF", "Dahua", 20),
    // Axis
    ("00:40:8C", "Axis", 20),
    ("AC:CC:8E", "Axis", 20),
];

/// ARP scan result for a single host
#[derive(Debug, Clone)]
pub struct ArpScanResult {
    pub ip: IpAddr,
    pub mac: String,
    pub vendor: Option<String>,
}

/// Check if target subnet is on the same L2 network as local interface
pub fn is_local_subnet(target_cidr: &str, local_ip: &str) -> bool {
    // Parse target CIDR
    let target_parts: Vec<&str> = target_cidr.split('/').collect();
    if target_parts.is_empty() {
        return false;
    }
    let target_network = target_parts[0];

    // Extract /24 subnet from both
    let target_octets: Vec<&str> = target_network.split('.').collect();
    let local_octets: Vec<&str> = local_ip.split('.').collect();

    if target_octets.len() >= 3 && local_octets.len() >= 3 {
        // Compare first 3 octets for /24
        target_octets[0] == local_octets[0]
            && target_octets[1] == local_octets[1]
            && target_octets[2] == local_octets[2]
    } else {
        false
    }
}

/// Get local IP address for the default interface
pub async fn get_local_ip() -> Option<String> {
    let output = Command::new("ip")
        .args(["route", "get", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse: "1.0.0.0 via 192.168.125.1 dev enp2s0 src 192.168.125.246 uid 1000"
    for part in stdout.split_whitespace() {
        if part.contains('.') && !part.starts_with("1.") {
            // Check if it looks like an IP
            if part.parse::<Ipv4Addr>().is_ok() {
                // Skip gateway, get src IP
                if let Some(idx) = stdout.find("src ") {
                    let after_src = &stdout[idx + 4..];
                    if let Some(ip) = after_src.split_whitespace().next() {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Perform ARP scan on a subnet (requires root/sudo)
/// Returns list of (IP, MAC) pairs for responsive hosts
pub async fn arp_scan_subnet(cidr: &str, interface: Option<&str>) -> Vec<ArpScanResult> {
    let mut cmd = Command::new("sudo");
    cmd.arg("arp-scan");

    if let Some(iface) = interface {
        cmd.arg("--interface").arg(iface);
    }

    cmd.arg(cidr);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, "arp-scan failed");
            return Vec::new();
        }
    };

    if !output.status.success() {
        tracing::warn!("arp-scan returned non-zero status");
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        // Parse: "192.168.125.66\t6c:c8:40:8c:a3:e0\t(Unknown)"
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            if let Ok(ip) = parts[0].parse::<Ipv4Addr>() {
                let mac = parts[1].to_uppercase();
                let vendor = lookup_oui(&mac);
                results.push(ArpScanResult {
                    ip: IpAddr::V4(ip),
                    mac,
                    vendor,
                });
            }
        }
    }

    tracing::info!(
        cidr = %cidr,
        hosts_found = results.len(),
        "ARP scan complete"
    );

    results
}

/// Score thresholds
/// Low threshold to capture potential cameras that need credentials
/// 443/8443 only = 20 points, so 15 captures these
pub const SCORE_THRESHOLD_VERIFY: u32 = 15;

/// Host discovery result
#[derive(Debug, Clone)]
pub struct HostResult {
    pub ip: IpAddr,
    pub alive: bool,
    pub method: DiscoveryMethod,
}

/// Discovery method used
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryMethod {
    TcpConnect,
    Known,
    Timeout,
}

/// Port scan result
#[derive(Debug, Clone)]
pub struct PortScanResult {
    pub ip: IpAddr,
    pub port: u16,
    pub open: bool,
    pub latency_ms: Option<u64>,
}

/// Probe result with detailed status
#[derive(Debug, Clone, Copy, Default)]
pub enum ProbeResult {
    #[default]
    NotTested,           // 未試行
    Success,             // 応答あり
    AuthRequired,        // 認証が必要（401等）
    Timeout,             // タイムアウト
    Refused,             // 接続拒否
    NoResponse,          // 接続はできたが応答なし
    Error,               // その他エラー
}

/// Device evidence for scoring
#[derive(Debug, Clone)]
pub struct DeviceEvidence {
    pub ip: IpAddr,
    pub open_ports: Vec<u16>,
    pub mac: Option<String>,
    pub oui_vendor: Option<String>,
    pub onvif_found: bool,
    pub ssdp_found: bool,
    pub mdns_found: bool,
    /// ONVIF probe詳細結果
    pub onvif_result: ProbeResult,
    /// RTSP probe詳細結果
    pub rtsp_result: ProbeResult,
}

impl DeviceEvidence {
    pub fn new(ip: IpAddr) -> Self {
        Self {
            ip,
            open_ports: Vec::new(),
            mac: None,
            oui_vendor: None,
            onvif_found: false,
            ssdp_found: false,
            mdns_found: false,
            onvif_result: ProbeResult::NotTested,
            rtsp_result: ProbeResult::NotTested,
        }
    }
}


/// Parse CIDR notation to IP list
pub fn parse_cidr(cidr: &str) -> Result<Vec<IpAddr>, String> {
    // Handle single IP
    if !cidr.contains('/') {
        return cidr
            .parse::<IpAddr>()
            .map(|ip| vec![ip])
            .map_err(|e| format!("Invalid IP: {}", e));
    }

    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid CIDR format: {}", cidr));
    }

    let base_ip: Ipv4Addr = parts[0]
        .parse()
        .map_err(|e| format!("Invalid IP: {}", e))?;
    let prefix: u8 = parts[1]
        .parse()
        .map_err(|e| format!("Invalid prefix: {}", e))?;

    if prefix > 32 {
        return Err(format!("Invalid prefix: {} (must be 0-32)", prefix));
    }

    // Calculate range
    let base_u32 = u32::from(base_ip);
    let mask = if prefix == 0 {
        0
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };
    let network = base_u32 & mask;
    let broadcast = network | !mask;

    // Generate IPs (skip network and broadcast for /24 and smaller)
    let mut ips = Vec::new();
    let start = if prefix >= 24 { network + 1 } else { network };
    let end = if prefix >= 24 { broadcast - 1 } else { broadcast };

    for ip_u32 in start..=end {
        ips.push(IpAddr::V4(Ipv4Addr::from(ip_u32)));
    }

    Ok(ips)
}

/// Stage 1: Host Discovery using TCP connect
pub async fn discover_host(ip: IpAddr, timeout_ms: u32) -> HostResult {
    // Try common camera ports for quick discovery
    let quick_ports = [554, 80, 443, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    for port in quick_ports {
        let addr = SocketAddr::new(ip, port);
        match timeout(timeout_dur, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => {
                return HostResult {
                    ip,
                    alive: true,
                    method: DiscoveryMethod::TcpConnect,
                };
            }
            Ok(Err(_)) => {
                // Connection refused = host is alive but port closed
                return HostResult {
                    ip,
                    alive: true,
                    method: DiscoveryMethod::TcpConnect,
                };
            }
            Err(_) => {
                // Timeout, try next port
                continue;
            }
        }
    }

    HostResult {
        ip,
        alive: false,
        method: DiscoveryMethod::Timeout,
    }
}

/// Stage 3: Port scan single port
pub async fn scan_port(ip: IpAddr, port: u16, timeout_ms: u32) -> PortScanResult {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);
    let start = std::time::Instant::now();

    match timeout(timeout_dur, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => PortScanResult {
            ip,
            port,
            open: true,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Ok(Err(_)) | Err(_) => PortScanResult {
            ip,
            port,
            open: false,
            latency_ms: None,
        },
    }
}

/// Stage 3: Scan all ports for a host
pub async fn scan_ports(ip: IpAddr, ports: &[u16], timeout_ms: u32) -> Vec<PortScanResult> {
    let mut results = Vec::new();

    for &port in ports {
        let result = scan_port(ip, port, timeout_ms).await;
        results.push(result);
    }

    results
}

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
    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
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

/// Stage 4: RTSP OPTIONS probe (no auth required)
pub async fn probe_rtsp(ip: IpAddr, port: u16, timeout_ms: u32) -> bool {
    probe_rtsp_detailed(ip, port, timeout_ms).await.0
}

/// Stage 4: RTSP OPTIONS probe with detailed result
pub async fn probe_rtsp_detailed(ip: IpAddr, port: u16, timeout_ms: u32) -> (bool, ProbeResult) {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let connect_result = timeout(timeout_dur, TcpStream::connect(addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => return (false, ProbeResult::Refused),
        Err(_) => return (false, ProbeResult::Timeout),
    };

    // Send RTSP OPTIONS
    let options_req = format!(
        "OPTIONS rtsp://{}:{} RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: IpcamScan/1.0\r\n\r\n",
        ip, port
    );

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    if let Err(_) = stream.write_all(options_req.as_bytes()).await {
        return (false, ProbeResult::Error);
    }

    let mut buf = [0u8; 1024];
    match timeout(timeout_dur, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            if response.contains("RTSP/1.0 200") || response.contains("200 OK") {
                (true, ProbeResult::Success)
            } else if response.contains("401") || response.contains("Unauthorized") {
                (false, ProbeResult::AuthRequired)
            } else if response.contains("RTSP/1.0") {
                // RTSP応答あり、ただし成功ではない
                (false, ProbeResult::NoResponse)
            } else {
                (false, ProbeResult::NoResponse)
            }
        }
        Ok(Ok(_)) => (false, ProbeResult::NoResponse),
        Ok(Err(_)) => (false, ProbeResult::Error),
        Err(_) => (false, ProbeResult::Timeout),
    }
}

/// Stage 5: Calculate score from evidence
pub fn calculate_score(evidence: &DeviceEvidence) -> u32 {
    let mut score = 0u32;

    // Port weights
    for &port in &evidence.open_ports {
        for &(p, weight) in PORT_WEIGHTS {
            if port == p {
                score += weight;
                break;
            }
        }
    }

    // OUI vendor bonus
    if let Some(ref vendor) = evidence.oui_vendor {
        let vendor_upper = vendor.to_uppercase();
        if vendor_upper.contains("TP-LINK") || vendor_upper.contains("TPLINK") {
            score += 20;
        } else if vendor_upper.contains("GOOGLE") {
            score += 20;
        }
    }

    // Discovery protocol bonuses
    if evidence.onvif_found {
        score += 50;
    }
    if evidence.ssdp_found {
        score += 20;
    }
    if evidence.mdns_found {
        score += 20;
    }

    score
}

/// Stage 6: Verify RTSP with credentials
pub async fn verify_rtsp(
    ip: IpAddr,
    port: u16,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<String> {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let connect_result = timeout(timeout_dur, TcpStream::connect(addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        _ => return None,
    };

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Common RTSP paths for Tapo cameras
    let paths = ["/stream1", "/stream2", "/h264_stream"];

    for path in paths {
        // Try with Basic auth
        let auth = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", username, password),
        );

        let describe_req = format!(
            "DESCRIBE rtsp://{}:{}{} RTSP/1.0\r\n\
             CSeq: 2\r\n\
             Authorization: Basic {}\r\n\
             User-Agent: IpcamScan/1.0\r\n\r\n",
            ip, port, path, auth
        );

        if let Err(_) = stream.write_all(describe_req.as_bytes()).await {
            return None;
        }

        let mut buf = [0u8; 2048];
        match timeout(timeout_dur, stream.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => {
                let response = String::from_utf8_lossy(&buf[..n]);
                if response.contains("200 OK") {
                    return Some(format!("rtsp://{}:{}{}", ip, port, path));
                }
            }
            _ => continue,
        }
    }

    None
}

/// Lookup OUI vendor from MAC address
pub fn lookup_oui(mac: &str) -> Option<String> {
    let mac_upper = mac.to_uppercase().replace(['-', ':'], "");
    let oui = if mac_upper.len() >= 6 {
        format!(
            "{}:{}:{}",
            &mac_upper[0..2],
            &mac_upper[2..4],
            &mac_upper[4..6]
        )
    } else {
        return None;
    };

    for &(prefix, vendor, _) in OUI_CAMERA_VENDORS {
        if oui == prefix {
            return Some(vendor.to_string());
        }
    }

    None
}

/// ONVIF device information retrieved via GetDeviceInformation
#[derive(Debug, Clone, Default)]
pub struct OnvifDeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
    pub mac_address: Option<String>,
}

/// Generate WS-Security UsernameToken Digest header for ONVIF authentication
fn generate_ws_security_header(username: &str, password: &str) -> String {
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
        r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
      <wsse:UsernameToken>
        <wsse:Username>{}</wsse:Username>
        <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</wsse:Password>
        <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</wsse:Nonce>
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
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
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
async fn probe_onvif_mac(
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
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Header>
    {}
  </s:Header>
  <s:Body>
    <GetNetworkInterfaces xmlns="http://www.onvif.org/ver10/device/wsdl"/>
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

/// Extract XML value with namespace-agnostic matching
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    // Try with common ONVIF namespace prefix first
    let prefixed_patterns = [
        format!("<tds:{}>", tag),
        format!("<tt:{}>", tag),
        format!("<:{}>", tag),
    ];

    for pattern in &prefixed_patterns {
        if let Some(start) = xml.find(pattern.as_str()) {
            let content_start = start + pattern.len();
            if let Some(end) = xml[content_start..].find("</") {
                let value = xml[content_start..content_start + end].trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }

    // Try without namespace prefix
    let pattern = format!(":{}>" , tag);
    if let Some(start) = xml.find(&pattern) {
        let content_start = start + pattern.len();
        if let Some(end) = xml[content_start..].find("</") {
            let value = xml[content_start..content_start + end].trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    // Final fallback: any tag ending with the name
    let simple_pattern = format!("<{}>", tag);
    if let Some(start) = xml.find(&simple_pattern) {
        let content_start = start + simple_pattern.len();
        let close_pattern = format!("</{}>", tag);
        if let Some(end) = xml[content_start..].find(&close_pattern) {
            let value = xml[content_start..content_start + end].trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cidr_single_ip() {
        let result = parse_cidr("192.168.1.1").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_cidr_24() {
        let result = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(result.len(), 254); // Excluding network and broadcast
    }

    #[test]
    fn test_parse_cidr_30() {
        let result = parse_cidr("192.168.1.0/30").unwrap();
        assert_eq!(result.len(), 2); // 2 usable IPs
    }

    #[test]
    fn test_calculate_score() {
        let evidence = DeviceEvidence {
            ip: "192.168.1.1".parse().unwrap(),
            open_ports: vec![554, 80],
            mac: None,
            oui_vendor: Some("TP-LINK".to_string()),
            onvif_found: true,
            ssdp_found: false,
            mdns_found: false,
        };

        let score = calculate_score(&evidence);
        // 554=30 + 80=10 + TP-LINK=20 + ONVIF=50 = 110
        assert_eq!(score, 110);
    }

    #[test]
    fn test_lookup_oui() {
        assert_eq!(lookup_oui("70:5A:0F:AA:BB:CC"), Some("TP-LINK".to_string()));
        assert_eq!(lookup_oui("F4:F5:D8:11:22:33"), Some("Google".to_string()));
        assert_eq!(lookup_oui("00:00:00:00:00:00"), None);
    }
}
