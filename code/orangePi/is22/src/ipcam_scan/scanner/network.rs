use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Stdio;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::timeout;

use super::lookup_oui;
use super::PORT_WEIGHTS;

/// ARP scan result
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
