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

    // Check for Locally Administered Address (LLA)
    // Second bit of first byte = 1 means locally administered
    // Common patterns: 02:xx:xx, 06:xx:xx, 0A:xx:xx, 0E:xx:xx, etc.
    if mac_upper.len() >= 2 {
        if let Ok(first_byte) = u8::from_str_radix(&mac_upper[0..2], 16) {
            if (first_byte & 0x02) != 0 {
                return Some("LLA:モバイルデバイス".to_string());
            }
        }
    }

    for &(prefix, vendor, _) in OUI_CAMERA_VENDORS {
        if oui == prefix {
            return Some(vendor.to_string());
        }
    }

    None
}
