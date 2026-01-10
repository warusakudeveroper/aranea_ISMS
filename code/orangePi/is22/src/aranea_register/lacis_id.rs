//! LacisID Generation for is22
//!
//! Phase 1: AraneaRegister (Issue #114)
//! DD01_AraneaRegister.md準拠
//!
//! ## LacisID形式
//! ```text
//! [Prefix=3][ProductType=022][MAC=12桁][ProductCode=0000] = 20桁
//!            │                │         │
//!            │                │         └─ 追い番なしのため固定
//!            │                └─ コロン除去、大文字化
//!            └─ is22 Paraclate Server
//! ```
//!
//! ## 例
//! MAC=AA:BB:CC:DD:EE:FF → LacisID=3022AABBCCDDEEFF0000

use super::types::{MAC_LENGTH, PREFIX, PRODUCT_CODE, PRODUCT_TYPE};
use std::io;

/// MACアドレスからLacisIDを生成
///
/// # Arguments
/// * `mac` - MACアドレス（任意形式: AA:BB:CC:DD:EE:FF, AA-BB-CC-DD-EE-FF, AABBCCDDEEFF）
///
/// # Returns
/// 20桁のLacisID文字列
///
/// # Panics
/// MACアドレスが不正な場合（12桁の16進数でない場合）
pub fn generate_lacis_id(mac: &str) -> String {
    // MACアドレスからコロン/ハイフンを除去し大文字化
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    assert_eq!(
        mac_clean.len(),
        MAC_LENGTH,
        "Invalid MAC address length: expected {}, got {}",
        MAC_LENGTH,
        mac_clean.len()
    );

    format!("{}{}{}{}", PREFIX, PRODUCT_TYPE, mac_clean, PRODUCT_CODE)
}

/// MACアドレスからLacisIDを生成（Result版）
///
/// # Arguments
/// * `mac` - MACアドレス
///
/// # Returns
/// Ok: 20桁のLacisID文字列
/// Err: MACアドレスが不正な場合
pub fn try_generate_lacis_id(mac: &str) -> Result<String, String> {
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    if mac_clean.len() != MAC_LENGTH {
        return Err(format!(
            "Invalid MAC address: expected {} hex digits, got {}",
            MAC_LENGTH,
            mac_clean.len()
        ));
    }

    Ok(format!(
        "{}{}{}{}",
        PREFIX, PRODUCT_TYPE, mac_clean, PRODUCT_CODE
    ))
}

/// システムのプライマリMACアドレスを取得
///
/// Linux環境ではネットワークインターフェースからMACを取得
pub fn get_primary_mac_address() -> io::Result<String> {
    #[cfg(target_os = "linux")]
    {
        // /sys/class/net からMACアドレスを取得
        // 優先順位: eth0 > enp* > wlan0 > その他
        let interfaces = ["eth0", "enp0s3", "enp0s31f6", "wlan0", "wlp*"];

        for iface in &interfaces {
            let path = format!("/sys/class/net/{}/address", iface);
            if let Ok(mac) = std::fs::read_to_string(&path) {
                let mac = mac.trim().to_uppercase();
                if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                    tracing::info!("Found MAC address from {}: {}", iface, mac);
                    return Ok(mac);
                }
            }
        }

        // どのインターフェースからも取得できない場合はディレクトリをスキャン
        let net_dir = std::fs::read_dir("/sys/class/net")?;
        for entry in net_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // loopbackは除外
            if name_str == "lo" {
                continue;
            }

            let path = format!("/sys/class/net/{}/address", name_str);
            if let Ok(mac) = std::fs::read_to_string(&path) {
                let mac = mac.trim().to_uppercase();
                if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                    tracing::info!("Found MAC address from {}: {}", name_str, mac);
                    return Ok(mac);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No valid network interface found",
        ))
    }

    #[cfg(target_os = "macos")]
    {
        // macOSではifconfigコマンドからMACを取得
        use std::process::Command;

        let output = Command::new("ifconfig").arg("en0").output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.contains("ether ") {
                if let Some(mac) = line.split_whitespace().nth(1) {
                    let mac = mac.to_uppercase();
                    tracing::info!("Found MAC address from en0: {}", mac);
                    return Ok(mac);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find MAC address from en0",
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "MAC address retrieval not supported on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lacis_id_with_colons() {
        assert_eq!(
            generate_lacis_id("AA:BB:CC:DD:EE:FF"),
            "3022AABBCCDDEEFF0000"
        );
    }

    #[test]
    fn test_generate_lacis_id_with_hyphens() {
        assert_eq!(
            generate_lacis_id("AA-BB-CC-DD-EE-FF"),
            "3022AABBCCDDEEFF0000"
        );
    }

    #[test]
    fn test_generate_lacis_id_lowercase() {
        assert_eq!(
            generate_lacis_id("aabbccddeeff"),
            "3022AABBCCDDEEFF0000"
        );
    }

    #[test]
    fn test_generate_lacis_id_mixed_case() {
        assert_eq!(
            generate_lacis_id("AaBbCcDdEeFf"),
            "3022AABBCCDDEEFF0000"
        );
    }

    #[test]
    fn test_generate_lacis_id_length() {
        let lacis_id = generate_lacis_id("AA:BB:CC:DD:EE:FF");
        assert_eq!(lacis_id.len(), 20);
    }

    #[test]
    fn test_try_generate_lacis_id_valid() {
        let result = try_generate_lacis_id("AA:BB:CC:DD:EE:FF");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3022AABBCCDDEEFF0000");
    }

    #[test]
    fn test_try_generate_lacis_id_invalid_short() {
        let result = try_generate_lacis_id("AA:BB:CC");
        assert!(result.is_err());
    }

    #[test]
    fn test_try_generate_lacis_id_invalid_chars() {
        let result = try_generate_lacis_id("GG:HH:II:JJ:KK:LL");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Invalid MAC address length")]
    fn test_generate_lacis_id_invalid_panics() {
        generate_lacis_id("invalid");
    }
}
