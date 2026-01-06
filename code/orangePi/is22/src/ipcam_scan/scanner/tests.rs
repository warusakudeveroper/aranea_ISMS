#[cfg(test)]
mod tests {
    use super::{calculate_score, lookup_oui, parse_cidr, DeviceEvidence, ProbeResult};

    #[test]
    fn test_parse_cidr_single_ip() {
        let result = parse_cidr("192.168.1.1").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_cidr_24() {
        let result = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(result.len(), 254); // Excluding network and broadcast
    }

    #[test]
    fn test_parse_cidr_30() {
        let result = parse_cidr("192.168.1.0/30").unwrap();
        assert_eq!(result.len(), 2); // 2 usable IPs
    }

    #[test]
    fn test_calculate_score() {
        let evidence = DeviceEvidence {
            ip: "192.168.1.1".parse().unwrap(),
            open_ports: vec![554, 80],
            mac: None,
            oui_vendor: Some("TP-LINK".to_string()),
            onvif_found: true,
            ssdp_found: false,
            mdns_found: false,
            onvif_result: ProbeResult::NotTested,
            rtsp_result: ProbeResult::NotTested,
        };

        let score = calculate_score(&evidence);
        // 554=30 + 80=10 + TP-LINK=20 + ONVIF=50 = 110
        assert_eq!(score, 110);
    }

    #[test]
    fn test_lookup_oui() {
        assert_eq!(lookup_oui("70:5A:0F:AA:BB:CC"), Some("TP-LINK".to_string()));
        assert_eq!(lookup_oui("F4:F5:D8:11:22:33"), Some("Google".to_string()));
        assert_eq!(lookup_oui("00:00:00:00:00:00"), None);
    }
}
