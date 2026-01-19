//! LostCamTracker 型定義
//!
//! 公開APIで使用する型を定義

use serde::{Deserialize, Serialize};

/// カメラステータス（API応答用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraTrackingStatus {
    pub camera_id: String,
    pub name: String,
    pub ip_address: String,
    pub status: TrackingStatusType,
    pub last_healthy_at: Option<String>,
    pub error_duration_minutes: Option<i64>,
    pub arp_scan_result: Option<String>,
    pub arp_scan_attempted_at: Option<String>,
    pub ip_relocation_count: i32,
    pub can_auto_track: bool,
}

/// 追跡ステータス種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingStatusType {
    /// 正常動作中
    Healthy,
    /// 異常検出（閾値未満）
    Unhealthy,
    /// ロスト（閾値超過、追跡対象）
    Lost,
    /// 追跡中（ARPスキャン実行中）
    Tracking,
    /// 手動スキャン推奨（VPN越し等）
    ManualRequired,
}

/// IP変更追跡結果（API応答用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRelocationRecord {
    pub id: i64,
    pub camera_id: String,
    pub camera_name: Option<String>,
    pub old_ip: String,
    pub new_ip: String,
    pub detection_method: String,
    pub scanner_device_id: Option<i32>,
    pub relocated_at: String,
}

/// ARPスキャナデバイス（API応答用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArpScannerDevice {
    pub id: i32,
    pub name: String,
    pub api_url: String,
    pub subnets: Option<Vec<String>>,
    pub enabled: bool,
    pub last_check_at: Option<String>,
    pub last_check_status: Option<String>,
}

/// LostCamTracker設定（API応答用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostCamTrackerSettings {
    pub enabled: bool,
    pub threshold_minutes: u32,
    pub retry_minutes: u32,
    pub local_subnets: Vec<String>,
    pub arp_scan_available: bool,
}

/// 追跡サイクル結果（API応答用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingCycleResult {
    pub timestamp: String,
    pub lost_cameras_detected: i32,
    pub cameras_relocated: i32,
    pub relocations: Vec<IpRelocationRecord>,
    pub errors: Vec<String>,
}

/// ロストカメラ一覧応答
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostCamerasResponse {
    pub cameras: Vec<CameraTrackingStatus>,
    pub total_lost: i32,
    pub auto_trackable: i32,
    pub manual_required: i32,
}

/// MAC正規化ユーティリティ
pub fn normalize_mac(mac: &str) -> String {
    mac.to_uppercase()
        .replace(":", "")
        .replace("-", "")
        .replace(".", "")
}

/// lacis_idからMACを抽出
///
/// lacis_id形式: [Prefix=1][ProductType=3][MAC=12][ProductCode=4] = 20文字
/// MACは位置4-15（0-indexed）
pub fn extract_mac_from_lacis_id(lacis_id: &str) -> Option<String> {
    if lacis_id.len() >= 16 {
        Some(lacis_id[4..16].to_uppercase())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mac() {
        assert_eq!(normalize_mac("a8:42:a1:b9:53:23"), "A842A1B95323");
        assert_eq!(normalize_mac("A8-42-A1-B9-53-23"), "A842A1B95323");
        assert_eq!(normalize_mac("a842a1b95323"), "A842A1B95323");
        assert_eq!(normalize_mac("a842.a1b9.5323"), "A842A1B95323");
    }

    #[test]
    fn test_extract_mac_from_lacis_id() {
        // 例: lacis_id = "1234A842A1B953230001"
        //     位置:      0123456789012345678901
        //     MAC部分:       ^^^^^^^^^^^^
        assert_eq!(
            extract_mac_from_lacis_id("1234A842A1B953230001"),
            Some("A842A1B95323".to_string())
        );

        // 短すぎる場合
        assert_eq!(extract_mac_from_lacis_id("1234A842"), None);
    }
}
