//! Utilities for IpcamScan

use std::net::Ipv4Addr;

use super::scanner;
use super::types::{
    CameraFamily, ConnectionStatus, DetectionReason, DeviceType, PortState, PortStatus,
    SuggestedAction,
};
use super::ProbeResult;

/// Default scan ports
pub fn default_ports() -> Vec<u16> {
    vec![554, 2020, 80, 443, 8000, 8080, 8443, 8554]
}

/// Extract /24 subnet from an IP string
pub fn extract_subnet(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.{}.0/24", parts[0], parts[1], parts[2])
    } else {
        ip.to_string()
    }
}

/// Check if an IP address falls within a CIDR subnet
pub fn ip_in_cidr(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network_ip: Ipv4Addr = match parts[0].parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };

    let prefix: u8 = match parts[1].parse() {
        Ok(p) if p <= 32 => p,
        _ => return false,
    };

    let device_ip: Ipv4Addr = match ip.parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };

    // Calculate network mask
    let mask = if prefix == 0 {
        0u32
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };

    let network_u32 = u32::from(network_ip);
    let device_u32 = u32::from(device_ip);

    // Check if device IP is within the subnet
    (device_u32 & mask) == (network_u32 & mask)
}

/// Map score to confidence (0-100)
pub fn calculate_confidence(score: u32) -> u8 {
    match score {
        0..=39 => 0,
        40..=59 => 30,
        60..=79 => 50,
        80..=99 => 70,
        100..=119 => 80,
        _ => 90,
    }
}

/// Determine camera family from device evidence
pub fn determine_camera_family(evidence: &scanner::DeviceEvidence) -> CameraFamily {
    if let Some(ref vendor) = evidence.oui_vendor {
        let vendor_upper = vendor.to_uppercase();
        if vendor_upper.contains("TP-LINK") || vendor_upper.contains("TPLINK") {
            if evidence.onvif_found && evidence.open_ports.contains(&2020) {
                CameraFamily::Tapo
            } else if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&2020) {
                CameraFamily::Tapo
            } else {
                CameraFamily::Other
            }
        } else if vendor_upper.contains("GOOGLE") {
            CameraFamily::Nest
        } else if vendor_upper.contains("AXIS") {
            CameraFamily::Axis
        } else if vendor_upper.contains("HIKVISION") || vendor_upper.contains("HIKV") {
            CameraFamily::Hikvision
        } else if vendor_upper.contains("DAHUA") {
            CameraFamily::Dahua
        } else {
            CameraFamily::Other
        }
    } else if evidence.onvif_found {
        if evidence.open_ports.contains(&2020) {
            CameraFamily::Tapo
        } else {
            CameraFamily::Other
        }
    } else if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&2020) {
        CameraFamily::Tapo
    } else {
        CameraFamily::Unknown
    }
}

/// Generate user-friendly DetectionReason from device evidence
pub fn generate_detection_reason(evidence: &scanner::DeviceEvidence) -> DetectionReason {
    // Convert ProbeResult to ConnectionStatus
    let onvif_status = match evidence.onvif_result {
        ProbeResult::NotTested => ConnectionStatus::NotTested,
        ProbeResult::Success => ConnectionStatus::Success,
        ProbeResult::AuthRequired => ConnectionStatus::AuthRequired,
        ProbeResult::Timeout => ConnectionStatus::Timeout,
        ProbeResult::Refused => ConnectionStatus::Refused,
        ProbeResult::NoResponse => ConnectionStatus::PortOpenOnly,
        ProbeResult::Error => ConnectionStatus::Unknown,
    };

    let rtsp_status = match evidence.rtsp_result {
        ProbeResult::NotTested => ConnectionStatus::NotTested,
        ProbeResult::Success => ConnectionStatus::Success,
        ProbeResult::AuthRequired => ConnectionStatus::AuthRequired,
        ProbeResult::Timeout => ConnectionStatus::Timeout,
        ProbeResult::Refused => ConnectionStatus::Refused,
        ProbeResult::NoResponse => ConnectionStatus::PortOpenOnly,
        ProbeResult::Error => ConnectionStatus::Unknown,
    };

    // Collect camera-related open ports
    let camera_related_ports: &[u16] = &[554, 2020, 8554, 80, 443, 8000, 8080, 8443];
    let camera_ports: Vec<u16> = evidence
        .open_ports
        .iter()
        .filter(|p| camera_related_ports.contains(p))
        .cloned()
        .collect();

    let (device_type, user_message, suggested_action) = if evidence.onvif_found
        && (matches!(evidence.rtsp_result, ProbeResult::Success)
            || evidence.open_ports.contains(&554))
    {
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (ONVIF/RTSP応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if evidence.onvif_found {
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (ONVIF応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if matches!(evidence.rtsp_result, ProbeResult::Success) {
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (RTSP応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if matches!(evidence.onvif_result, ProbeResult::AuthRequired)
        || matches!(evidence.rtsp_result, ProbeResult::AuthRequired)
    {
        let vendor_hint = if let Some(ref vendor) = evidence.oui_vendor {
            let vendor_upper = vendor.to_uppercase();
            if vendor_upper.contains("TP-LINK") {
                "Tapoカメラ"
            } else if vendor_upper.contains("GOOGLE") {
                "Nest/Googleカメラ"
            } else if vendor_upper.contains("HIKVISION") {
                "Hikvisionカメラ"
            } else if vendor_upper.contains("DAHUA") {
                "Dahuaカメラ"
            } else {
                "カメラ"
            }
        } else {
            "カメラ"
        };
        (
            DeviceType::CameraLikely,
            format!("{}の可能性あり (認証が必要)", vendor_hint),
            SuggestedAction::SetCredentials,
        )
    } else if let Some(ref vendor) = evidence.oui_vendor {
        let vendor_upper = vendor.to_uppercase();
        if vendor_upper.contains("TP-LINK") {
            if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&2020) {
                (
                    DeviceType::CameraLikely,
                    "Tapoカメラの可能性あり (クレデンシャル設定が必要)".to_string(),
                    SuggestedAction::SetCredentials,
                )
            } else if evidence.open_ports.contains(&80) || evidence.open_ports.contains(&443) {
                (
                    DeviceType::CameraPossible,
                    "TP-Linkデバイス検出 (カメラか要確認)".to_string(),
                    SuggestedAction::ManualCheck,
                )
            } else {
                (
                    DeviceType::NetworkDevice,
                    "TP-Linkネットワーク機器 (スイッチ/ルーター?)".to_string(),
                    SuggestedAction::Ignore,
                )
            }
        } else if vendor_upper.contains("GOOGLE") {
            (
                DeviceType::CameraLikely,
                "Nest/Googleカメラの可能性あり".to_string(),
                SuggestedAction::ManualCheck,
            )
        } else {
            (
                DeviceType::CameraPossible,
                format!("{}製デバイス検出", vendor),
                SuggestedAction::ManualCheck,
            )
        }
    } else if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&8554) {
        (
            DeviceType::CameraPossible,
            "RTSPポート検出 (カメラの可能性あり、クレデンシャル設定が必要)".to_string(),
            SuggestedAction::SetCredentials,
        )
    } else if evidence.open_ports.contains(&2020) {
        (
            DeviceType::CameraPossible,
            "ONVIFポート検出 (カメラの可能性あり、クレデンシャル設定が必要)".to_string(),
            SuggestedAction::SetCredentials,
        )
    } else if evidence.open_ports.contains(&8000) && evidence.open_ports.contains(&8080) {
        (
            DeviceType::NvrLikely,
            "NVR/録画機器の可能性あり".to_string(),
            SuggestedAction::ManualCheck,
        )
    } else if evidence.open_ports.contains(&443) || evidence.open_ports.contains(&8443) {
        (
            DeviceType::CameraPossible,
            "Web管理対応デバイス (カメラの可能性あり)".to_string(),
            SuggestedAction::ManualCheck,
        )
    } else {
        (
            DeviceType::OtherDevice,
            "その他のデバイス".to_string(),
            SuggestedAction::Ignore,
        )
    };

    DetectionReason {
        oui_match: evidence.oui_vendor.clone(),
        camera_ports,
        onvif_status,
        rtsp_status,
        device_type,
        user_message,
        suggested_action,
    }
}

/// パスワードバリエーション生成（よくある入力ミス対応）
pub fn generate_password_variations(password: &str) -> Vec<(String, String)> {
    let mut variations = Vec::new();

    // 1. original
    variations.push((password.to_string(), "original".to_string()));

    // 2. Toggle first char case
    if let Some(first) = password.chars().next() {
        let toggled = if first.is_lowercase() {
            first.to_uppercase().collect::<String>()
        } else {
            first.to_lowercase().collect::<String>()
        };
        let mut toggled_pass = toggled;
        toggled_pass.push_str(&password[first.len_utf8()..]);
        if toggled_pass != password {
            variations.push((toggled_pass, "first_char_toggle".to_string()));
        }
    }

    // 3. Append '@' if not present
    if !password.ends_with('@') {
        let mut appended = password.to_string();
        appended.push('@');
        variations.push((appended, "append_at".to_string()));
    }

    // 4. Remove trailing '@' if present
    if password.ends_with('@') {
        let mut removed = password.to_string();
        removed.pop();
        variations.push((removed, "remove_at".to_string()));
    }

    variations
}
