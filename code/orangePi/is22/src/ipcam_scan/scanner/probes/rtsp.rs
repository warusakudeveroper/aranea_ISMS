use std::net::IpAddr;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::super::ProbeResult;

/// Stage 4: RTSP OPTIONS probe (no auth required)
pub async fn probe_rtsp(ip: IpAddr, port: u16, timeout_ms: u32) -> bool {
    probe_rtsp_detailed(ip, port, timeout_ms).await.0
}

/// Stage 4: RTSP OPTIONS probe with detailed result
pub async fn probe_rtsp_detailed(ip: IpAddr, port: u16, timeout_ms: u32) -> (bool, ProbeResult) {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let connect_result = timeout(timeout_dur, TcpStream::connect(addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => return (false, ProbeResult::Refused),
        Err(_) => return (false, ProbeResult::Timeout),
    };

    // Send RTSP OPTIONS
    let options_req = format!(
        "OPTIONS rtsp://{}:{} RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: IpcamScan/1.0\r\n\r\n",
        ip, port
    );

    if let Err(_) = stream.write_all(options_req.as_bytes()).await {
        return (false, ProbeResult::Error);
    }

    let mut buf = [0u8; 1024];
    match timeout(timeout_dur, stream.read(&mut buf)).await {
        Ok(Ok(n)) if n > 0 => {
            let response = String::from_utf8_lossy(&buf[..n]);
            if response.contains("RTSP/1.0 200") || response.contains("200 OK") {
                (true, ProbeResult::Success)
            } else if response.contains("401") || response.contains("Unauthorized") {
                (false, ProbeResult::AuthRequired)
            } else if response.contains("RTSP/1.0") {
                // RTSP応答あり、ただし成功ではない
                (false, ProbeResult::NoResponse)
            } else {
                (false, ProbeResult::NoResponse)
            }
        }
        Ok(Ok(_)) => (false, ProbeResult::NoResponse),
        Ok(Err(_)) => (false, ProbeResult::Error),
        Err(_) => (false, ProbeResult::Timeout),
    }
}

/// Stage 6: Verify RTSP with credentials
pub async fn verify_rtsp(
    ip: IpAddr,
    port: u16,
    username: &str,
    password: &str,
    timeout_ms: u32,
) -> Option<String> {
    let addr = SocketAddr::new(ip, port);
    let timeout_dur = Duration::from_millis(timeout_ms as u64);

    let connect_result = timeout(timeout_dur, TcpStream::connect(addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        _ => return None,
    };

    // Common RTSP paths for Tapo cameras
    let paths = ["/stream1", "/stream2", "/h264_stream"];

    for path in paths {
        // Try with Basic auth
        let auth = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", username, password),
        );

        let describe_req = format!(
            "DESCRIBE rtsp://{}:{}{} RTSP/1.0\r\n\
             CSeq: 2\r\n\
             Authorization: Basic {}\r\n\
             User-Agent: IpcamScan/1.0\r\n\r\n",
            ip, port, path, auth
        );

        if let Err(_) = stream.write_all(describe_req.as_bytes()).await {
            return None;
        }

        let mut buf = [0u8; 2048];
        match timeout(timeout_dur, stream.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => {
                let response = String::from_utf8_lossy(&buf[..n]);
                if response.contains("200 OK") {
                    return Some(format!("rtsp://{}:{}{}", ip, port, path));
                }
            }
            _ => continue,
        }
    }

    None
}
