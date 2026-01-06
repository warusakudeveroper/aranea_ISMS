//! Scanner implementation for IpcamScan
//!
//! Multi-stage evidence-based camera discovery

mod port_weights;
mod oui_data;
mod probes;
mod network;

pub use port_weights::PORT_WEIGHTS;
pub use oui_data::{lookup_oui, OUI_CAMERA_VENDORS};
pub use probes::{
    probe_onvif,
    probe_onvif_capabilities,
    probe_onvif_detailed,
    probe_onvif_extended,
    probe_onvif_network_interfaces_full,
    probe_onvif_scopes,
    probe_onvif_with_auth,
    probe_rtsp,
    probe_rtsp_detailed,
    verify_rtsp,
    OnvifCapabilities,
    OnvifDeviceInfo,
    OnvifExtendedInfo,
    OnvifNetworkInterface,
    OnvifScopes,
};
pub use network::{
    arp_scan_subnet,
    calculate_score,
    discover_host,
    get_local_ip,
    is_local_subnet,
    parse_cidr,
    scan_port,
    scan_ports,
    ArpScanResult,
    DeviceEvidence,
    DiscoveryMethod,
    HostResult,
    PortScanResult,
    ProbeResult,
    SCORE_THRESHOLD_VERIFY,
};

#[cfg(test)]
mod tests;
