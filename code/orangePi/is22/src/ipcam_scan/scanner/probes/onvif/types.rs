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

/// ONVIF Scopes data from GetScopes
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OnvifScopes {
    pub name: Option<String>,
    pub hardware: Option<String>,
    pub profile: Option<String>,
    pub location: Option<String>,
    pub device_type: Option<String>,
    pub raw_scopes: Vec<String>,
}

/// ONVIF Network Interface data from GetNetworkInterfaces
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OnvifNetworkInterface {
    pub token: Option<String>,
    pub enabled: Option<bool>,
    pub hw_address: Option<String>,
    pub mtu: Option<i32>,
    pub ipv4_enabled: Option<bool>,
    pub ipv4_address: Option<String>,
    pub ipv4_prefix: Option<i32>,
    pub dhcp: Option<bool>,
}

/// ONVIF Capabilities data from GetCapabilities
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OnvifCapabilities {
    // Analytics
    pub analytics_support: bool,
    pub analytics_xaddr: Option<String>,
    // Device
    pub device_xaddr: Option<String>,
    pub device_io_support: bool,
    // Events
    pub events_support: bool,
    pub events_xaddr: Option<String>,
    pub ws_subscription_support: bool,
    pub ws_pullpoint_support: bool,
    // Imaging
    pub imaging_support: bool,
    pub imaging_xaddr: Option<String>,
    // Media
    pub media_support: bool,
    pub media_xaddr: Option<String>,
    pub rtsp_streaming: bool,
    // PTZ
    pub ptz_support: bool,
    pub ptz_xaddr: Option<String>,
}

/// Extended ONVIF data (all collected data)
#[derive(Debug, Clone, Default)]
pub struct OnvifExtendedInfo {
    pub device_info: OnvifDeviceInfo,
    pub scopes: Option<OnvifScopes>,
    pub network_interfaces: Vec<OnvifNetworkInterface>,
    pub capabilities: Option<OnvifCapabilities>,
}
