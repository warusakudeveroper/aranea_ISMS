//! Scanner implementation for IpcamScan
//!
//! Multi-stage evidence-based camera discovery

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

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
    ("70:5A:0F", "TP-LINK", 20),
    ("54:AF:97", "TP-LINK", 20),
    ("B0:A7:B9", "TP-LINK", 20),
    ("6C:5A:B0", "TP-LINK", 20),
    ("F4:F5:D8", "Google", 20),
    ("30:FD:38", "Google", 20),
    ("3C:22:FB", "Google", 20),
    ("18:D6:C7", "TP-LINK", 20),
];

/// Score thresholds
pub const SCORE_THRESHOLD_VERIFY: u32 = 40;

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
pub async fn probe_onvif(ip: IpAddr, timeout_ms: u32) -> bool {
    let ports = [80, 2020, 8080];
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    // ONVIF GetSystemDateAndTime - simplest unauthenticated call
    let soap_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
</s:Envelope>"#;

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

        if let Ok(resp) = result {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    if body.contains("SystemDateAndTime") || body.contains("UTCDateTime") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Stage 4: RTSP OPTIONS probe (no auth required)
pub async fn probe_rtsp(ip: IpAddr, port: u16, timeout_ms: u32) -> bool {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let connect_result = timeout(timeout_dur, TcpStream::connect(addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        _ => return false,
    };

    // Send RTSP OPTIONS
    let options_req = format!(
        "OPTIONS rtsp://{}:{} RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: IpcamScan/1.0\r\n\r\n",
        ip, port
    );

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    if let Err(_) = stream.write_all(options_req.as_bytes()).await {
        return false;
    }

    let mut buf = [0u8; 1024];
    match timeout(timeout_dur, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            response.contains("RTSP/1.0") || response.contains("200 OK")
        }
        _ => false,
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
