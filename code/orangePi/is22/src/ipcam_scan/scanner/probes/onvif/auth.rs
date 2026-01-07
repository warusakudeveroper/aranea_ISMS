// Re-export authentication functions from sibling modules
pub use super::auth_base::{generate_ws_security_header, probe_onvif_mac, probe_onvif_with_auth};
pub use super::auth_data::{
    probe_onvif_capabilities,
    probe_onvif_extended,
    probe_onvif_network_interfaces_full,
    probe_onvif_scopes,
};
