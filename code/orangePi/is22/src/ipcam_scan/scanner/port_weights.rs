/// Port weights for scoring
pub const PORT_WEIGHTS: &[(u16, u32)] = &[
    (554, 30),   // RTSP
    (2020, 30),  // ONVIF
    (80, 10),    // HTTP
    (443, 10),   // HTTPS
    (8000, 10),  // NVR
    (8080, 5),   // Alt HTTP
    (8443, 5),   // Alt HTTPS
    (8554, 5),   // Alt RTSP
];
