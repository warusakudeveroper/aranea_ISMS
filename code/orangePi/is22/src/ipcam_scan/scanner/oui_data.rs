use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use tokio::process::Command;
use std::process::Stdio;

/// Type alias for OUI lookup map
/// Key: OUI prefix in format "XX:XX:XX" (uppercase)
/// Value: Vendor name (e.g., "TP-LINK", "Google")
pub type OuiMap = HashMap<String, String>;

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
/// Use oui_map from CameraBrandService for vendor lookups
pub async fn arp_scan_subnet(cidr: &str, interface: Option<&str>, oui_map: Option<&OuiMap>) -> Vec<ArpScanResult> {
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
                let vendor = lookup_oui(&mac, oui_map);
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

// Note: HostResult, DiscoveryMethod, PortScanResult, ProbeResult are defined in network.rs
// SCORE_THRESHOLD_VERIFY is also defined in network.rs

/// Extract OUI prefix from MAC address
/// Returns OUI in format "XX:XX:XX" (uppercase)
pub fn extract_oui_prefix(mac: &str) -> Option<String> {
    let mac_upper = mac.to_uppercase().replace(['-', ':'], "");
    if mac_upper.len() >= 6 {
        Some(format!(
            "{}:{}:{}",
            &mac_upper[0..2],
            &mac_upper[2..4],
            &mac_upper[4..6]
        ))
    } else {
        None
    }
}

/// Check if MAC is Locally Administered Address (LLA)
/// Second bit of first byte = 1 means locally administered
/// Common patterns: 02:xx:xx, 06:xx:xx, 0A:xx:xx, 0E:xx:xx, etc.
pub fn is_locally_administered(mac: &str) -> bool {
    let mac_upper = mac.to_uppercase().replace(['-', ':'], "");
    if mac_upper.len() >= 2 {
        if let Ok(first_byte) = u8::from_str_radix(&mac_upper[0..2], 16) {
            return (first_byte & 0x02) != 0;
        }
    }
    false
}

/// Lookup OUI vendor from MAC address using provided OUI map
/// Falls back to LLA detection if OUI not found
pub fn lookup_oui(mac: &str, oui_map: Option<&OuiMap>) -> Option<String> {
    // Check for Locally Administered Address first
    if is_locally_administered(mac) {
        return Some("LLA:モバイルデバイス".to_string());
    }

    // Extract OUI prefix
    let oui = extract_oui_prefix(mac)?;

    // Lookup in provided map
    if let Some(map) = oui_map {
        if let Some(vendor) = map.get(&oui) {
            return Some(vendor.clone());
        }
    }

    None
}
