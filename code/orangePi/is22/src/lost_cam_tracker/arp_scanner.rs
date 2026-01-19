//! ARP Scanner - arp-scanコマンドのラッパー
//!
//! ## 重要: カメラに負荷をかけない
//!
//! このモジュールはARP (Layer 2) のみを使用する。
//! ONVIF/RTSP認証試行やポートスキャンは絶対に行わない。

use crate::Error;
use std::process::Command;
use tokio::sync::Semaphore;

/// ARPエントリ
#[derive(Debug, Clone)]
pub struct ArpEntry {
    pub ip: String,
    pub mac: String,
}

/// ARPスキャナ
pub struct ArpScanner {
    /// 同時実行制限（最大2並列）
    semaphore: Semaphore,
    /// arp-scanコマンドのパス
    arp_scan_path: String,
    /// ネットワークインターフェース
    interface: String,
}

impl ArpScanner {
    /// 新しいArpScannerを作成
    /// 環境変数 ARP_SCAN_INTERFACE でインターフェースを指定可能（デフォルト: 自動検出）
    pub fn new() -> Self {
        let interface = std::env::var("ARP_SCAN_INTERFACE")
            .unwrap_or_else(|_| Self::detect_default_interface());
        Self {
            semaphore: Semaphore::new(2),
            arp_scan_path: "arp-scan".to_string(),
            interface,
        }
    }

    /// デフォルトのネットワークインターフェースを検出
    fn detect_default_interface() -> String {
        // 優先順位: eth0 > enp* > ens* > eno*
        let candidates = ["eth0", "enp2s0", "enp3s0", "ens192", "eno1"];

        if let Ok(output) = std::process::Command::new("ip")
            .args(["link", "show"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for candidate in candidates {
                if stdout.contains(&format!("{}: <", candidate)) {
                    return candidate.to_string();
                }
            }
            // カスタム検出: 最初の非loインターフェース
            for line in stdout.lines() {
                if let Some(iface) = line.split(':').nth(1) {
                    let iface = iface.trim();
                    if !iface.is_empty() && iface != "lo" && !iface.contains('@') {
                        return iface.to_string();
                    }
                }
            }
        }

        // フォールバック
        "eth0".to_string()
    }

    /// カスタム設定でArpScannerを作成
    pub fn with_config(interface: &str) -> Self {
        Self {
            semaphore: Semaphore::new(2),
            arp_scan_path: "arp-scan".to_string(),
            interface: interface.to_string(),
        }
    }

    /// サブネットをARPスキャン
    ///
    /// ## 引数
    /// - `subnet`: スキャン対象サブネット（例: "192.168.125.0/24"）
    ///
    /// ## 戻り値
    /// - `Vec<ArpEntry>`: 検出されたIP/MACペア
    ///
    /// ## 注意
    /// - sudoが必要（arp-scanはraw socketを使用）
    /// - sudoersに設定済みの前提: `mijeosadmin ALL=(ALL) NOPASSWD: /usr/bin/arp-scan`
    pub async fn scan_subnet(&self, subnet: &str) -> Result<Vec<ArpEntry>, Error> {
        // 同時実行制限
        let _permit = self.semaphore.acquire().await
            .map_err(|e| Error::Internal(format!("Semaphore error: {}", e)))?;

        tracing::debug!(
            subnet = %subnet,
            interface = %self.interface,
            "Starting ARP scan"
        );

        // arp-scanコマンド実行
        let output = tokio::task::spawn_blocking({
            let interface = self.interface.clone();
            let subnet = subnet.to_string();
            let arp_scan_path = self.arp_scan_path.clone();
            move || {
                Command::new("sudo")
                    .args([&arp_scan_path, "--interface", &interface, &subnet, "--timeout", "1000"])
                    .output()
            }
        })
        .await
        .map_err(|e| Error::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| Error::Internal(format!("arp-scan execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // arp-scanがインストールされていない場合
            if stderr.contains("command not found") || stderr.contains("No such file") {
                tracing::warn!("arp-scan not installed, falling back to system ARP cache");
                return self.read_system_arp_cache().await;
            }
            // 権限エラー
            if stderr.contains("Operation not permitted") {
                return Err(Error::Internal(
                    "arp-scan requires sudo. Please configure: mijeosadmin ALL=(ALL) NOPASSWD: /usr/bin/arp-scan".to_string()
                ));
            }
            return Err(Error::Internal(format!("arp-scan failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries = self.parse_arp_scan_output(&stdout);

        tracing::info!(
            subnet = %subnet,
            device_count = entries.len(),
            "ARP scan completed"
        );

        Ok(entries)
    }

    /// arp-scan出力をパース
    ///
    /// 出力例:
    /// ```text
    /// 192.168.125.1    00:1a:2b:3c:4d:5e    TP-LINK TECHNOLOGIES
    /// 192.168.125.45   a8:42:a1:b9:53:23    TP-LINK TECHNOLOGIES
    /// ```
    fn parse_arp_scan_output(&self, output: &str) -> Vec<ArpEntry> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].contains('.') && parts[1].contains(':') {
                    let ip = parts[0].to_string();
                    // MAC正規化: 大文字化 + セパレータ除去
                    let mac = parts[1]
                        .to_uppercase()
                        .replace(":", "")
                        .replace("-", "");
                    Some(ArpEntry { ip, mac })
                } else {
                    None
                }
            })
            .collect()
    }

    /// システムのARPキャッシュを読み取る（フォールバック）
    ///
    /// arp-scanが使えない環境向けのフォールバック。
    /// キャッシュは古い可能性があるため、能動的スキャンが推奨。
    async fn read_system_arp_cache(&self) -> Result<Vec<ArpEntry>, Error> {
        let output = tokio::task::spawn_blocking(|| {
            Command::new("ip")
                .args(["neigh", "show"])
                .output()
        })
        .await
        .map_err(|e| Error::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| Error::Internal(format!("ip neigh show failed: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Internal("Failed to read ARP cache".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries = self.parse_ip_neigh_output(&stdout);

        tracing::info!(
            device_count = entries.len(),
            "Read system ARP cache (fallback)"
        );

        Ok(entries)
    }

    /// `ip neigh show`出力をパース
    ///
    /// 出力例:
    /// ```text
    /// 192.168.125.1 dev eth0 lladdr 00:1a:2b:3c:4d:5e REACHABLE
    /// 192.168.125.45 dev eth0 lladdr a8:42:a1:b9:53:23 STALE
    /// ```
    fn parse_ip_neigh_output(&self, output: &str) -> Vec<ArpEntry> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // IP dev interface lladdr MAC STATE
                if parts.len() >= 5 && parts[0].contains('.') && parts[3] == "lladdr" {
                    let ip = parts[0].to_string();
                    let mac = parts[4]
                        .to_uppercase()
                        .replace(":", "")
                        .replace("-", "");
                    Some(ArpEntry { ip, mac })
                } else {
                    None
                }
            })
            .collect()
    }

    /// arp-scanが利用可能かチェック
    pub async fn is_available(&self) -> bool {
        let result = tokio::task::spawn_blocking({
            let arp_scan_path = self.arp_scan_path.clone();
            move || {
                Command::new("which")
                    .arg(&arp_scan_path)
                    .output()
            }
        })
        .await;

        match result {
            Ok(Ok(output)) => output.status.success(),
            _ => false,
        }
    }
}

impl Default for ArpScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arp_scan_output() {
        let scanner = ArpScanner::new();
        let output = r#"
Interface: eth0, datalink type: EN10MB (Ethernet)
Starting arp-scan 1.9 with 256 hosts
192.168.125.1	00:1a:2b:3c:4d:5e	TP-LINK TECHNOLOGIES
192.168.125.45	a8:42:a1:b9:53:23	TP-LINK TECHNOLOGIES
192.168.125.62	dc:62:79:8d:08:ea	TP-LINK TECHNOLOGIES

3 packets received by filter, 0 packets dropped by kernel
"#;

        let entries = scanner.parse_arp_scan_output(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].ip, "192.168.125.1");
        assert_eq!(entries[0].mac, "001A2B3C4D5E");
        assert_eq!(entries[1].ip, "192.168.125.45");
        assert_eq!(entries[1].mac, "A842A1B95323");
    }

    #[test]
    fn test_parse_ip_neigh_output() {
        let scanner = ArpScanner::new();
        let output = r#"
192.168.125.1 dev eth0 lladdr 00:1a:2b:3c:4d:5e REACHABLE
192.168.125.45 dev eth0 lladdr a8:42:a1:b9:53:23 STALE
192.168.125.62 dev eth0 lladdr dc:62:79:8d:08:ea DELAY
fe80::1 dev eth0 lladdr 00:1a:2b:3c:4d:5e router REACHABLE
"#;

        let entries = scanner.parse_ip_neigh_output(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].ip, "192.168.125.1");
        assert_eq!(entries[0].mac, "001A2B3C4D5E");
    }
}
