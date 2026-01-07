use super::{calculate_score, parse_cidr, DeviceEvidence, ProbeResult, lookup_oui, OuiMap};

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
fn test_lookup_oui_with_map() {
    // Create a mock OUI map
    let mut oui_map: OuiMap = OuiMap::new();
    oui_map.insert("70:5A:0F".to_string(), "TP-Link / Tapo".to_string());
    oui_map.insert("F4:F5:D8".to_string(), "Google / Nest".to_string());
    oui_map.insert("C0:56:E3".to_string(), "Hikvision".to_string());

    // Test with OUI map
    assert_eq!(lookup_oui("70:5A:0F:AA:BB:CC", Some(&oui_map)), Some("TP-Link / Tapo".to_string()));
    assert_eq!(lookup_oui("F4:F5:D8:11:22:33", Some(&oui_map)), Some("Google / Nest".to_string()));
    assert_eq!(lookup_oui("C0:56:E3:00:00:00", Some(&oui_map)), Some("Hikvision".to_string()));

    // Test unknown OUI - returns None
    assert_eq!(lookup_oui("00:00:00:00:00:00", Some(&oui_map)), None);
}

#[test]
fn test_lookup_oui_without_map() {
    // Without OUI map, only LLA detection works
    // Regular OUIs return None
    assert_eq!(lookup_oui("70:5A:0F:AA:BB:CC", None), None);
    assert_eq!(lookup_oui("00:00:00:00:00:00", None), None);
}

#[test]
fn test_lookup_oui_lla_detection() {
    // Locally Administered Address detection (second bit of first byte = 1)
    // These should return "LLA:モバイルデバイス" regardless of OUI map
    assert_eq!(lookup_oui("02:00:00:00:00:00", None), Some("LLA:モバイルデバイス".to_string()));
    assert_eq!(lookup_oui("06:11:22:33:44:55", None), Some("LLA:モバイルデバイス".to_string()));
    assert_eq!(lookup_oui("0A:AA:BB:CC:DD:EE", None), Some("LLA:モバイルデバイス".to_string()));

    // Non-LLA addresses
    assert_eq!(lookup_oui("00:00:00:00:00:00", None), None); // Not LLA
    assert_eq!(lookup_oui("04:00:00:00:00:00", None), None); // Not LLA
}
