//! IpcamScan - Camera Auto-Discovery
//!
//! Issue #30 [IS22-GAP-004] IpcamScan implementation
//!
//! ## Responsibilities
//!
//! - Multi-stage evidence-based discovery
//! - Stage 0-6: Prep → Host → MAC → Port → Discovery → Score → Verify
//! - No brute-force, proper credentials only
//! - DB persistence for discovered devices

mod scanner;
mod types;

pub use types::*;

use crate::error::{Error, Result};
use scanner::{
    arp_scan_subnet, calculate_score, discover_host, get_local_ip, is_local_subnet, lookup_oui,
    parse_cidr, probe_onvif_detailed, probe_onvif_with_auth, probe_rtsp_detailed, scan_ports,
    ArpScanResult, DeviceEvidence, OnvifDeviceInfo, ProbeResult,
};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// IpcamScan service
pub struct IpcamScan {
    pool: MySqlPool,
    jobs: Arc<RwLock<HashMap<Uuid, ScanJob>>>,
}

impl IpcamScan {
    /// Create new IpcamScan with DB pool
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new scan job
    pub async fn create_job(&self, request: ScanJobRequest) -> ScanJob {
        let job = ScanJob {
            job_id: Uuid::new_v4(),
            targets: request.targets,
            mode: request.mode.unwrap_or_default(),
            ports: request.ports.unwrap_or_else(default_ports),
            timeout_ms: request.timeout_ms.unwrap_or(3000),
            concurrency: request.concurrency.unwrap_or(10),
            status: JobStatus::Queued,
            started_at: None,
            ended_at: None,
            summary: None,
            created_at: chrono::Utc::now(),
            logs: Vec::new(),
            current_phase: Some("Queued".to_string()),
            progress_percent: Some(0),
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job.job_id, job.clone());
        }

        tracing::info!(job_id = %job.job_id, "Scan job created");

        job
    }

    /// Add a log entry to a job
    async fn add_log(&self, job_id: &Uuid, entry: ScanLogEntry) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.logs.push(entry);
        }
    }

    /// Update job phase and progress
    async fn update_phase(&self, job_id: &Uuid, phase: &str, progress: u8) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.current_phase = Some(phase.to_string());
            job.progress_percent = Some(progress);
        }
    }

    /// Get job status
    pub async fn get_job(&self, job_id: &Uuid) -> Option<ScanJob> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<ScanJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Get discovered devices from DB
    pub async fn list_devices(&self, filter: DeviceFilter) -> Vec<ScannedDevice> {
        // Fetch all and filter in Rust (simple implementation)
        let result: std::result::Result<Vec<_>, _> = sqlx::query_as::<_, DbDevice>(
            "SELECT device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, \
             score, verified, status, manufacturer, model, firmware, family, \
             confidence, rtsp_uri, detection_json, first_seen, last_seen, \
             credential_status, credential_username, credential_password \
             FROM ipcamscan_devices ORDER BY score DESC, last_seen DESC"
        )
        .fetch_all(&self.pool)
        .await;

        match result {
            Ok(db_devices) => {
                db_devices
                    .into_iter()
                    .map(|d| d.into_scanned_device())
                    .filter(|d| {
                        if let Some(ref subnet) = filter.subnet {
                            if !d.subnet.contains(subnet) {
                                return false;
                            }
                        }
                        if let Some(ref family) = filter.family {
                            if &d.family != family {
                                return false;
                            }
                        }
                        if let Some(verified) = filter.verified {
                            if d.verified != verified {
                                return false;
                            }
                        }
                        if let Some(status) = filter.status {
                            if d.status != status {
                                return false;
                            }
                        }
                        true
                    })
                    .collect()
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to list devices from DB");
                Vec::new()
            }
        }
    }

    /// Get a single device by IP
    pub async fn get_device(&self, ip: &str) -> Option<ScannedDevice> {
        let result: std::result::Result<DbDevice, _> = sqlx::query_as(
            "SELECT device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, \
             score, verified, status, manufacturer, model, firmware, family, \
             confidence, rtsp_uri, detection_json, first_seen, last_seen, \
             credential_status, credential_username, credential_password \
             FROM ipcamscan_devices WHERE ip = ?"
        )
        .bind(ip)
        .fetch_one(&self.pool)
        .await;

        result.ok().map(|d| d.into_scanned_device())
    }

    /// Get a single device by ID
    pub async fn get_device_by_id(&self, device_id: &Uuid) -> Option<ScannedDevice> {
        let result: std::result::Result<DbDevice, _> = sqlx::query_as(
            "SELECT device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, \
             score, verified, status, manufacturer, model, firmware, family, \
             confidence, rtsp_uri, detection_json, first_seen, last_seen, \
             credential_status, credential_username, credential_password \
             FROM ipcamscan_devices WHERE device_id = ?"
        )
        .bind(device_id.to_string())
        .fetch_one(&self.pool)
        .await;

        result.ok().map(|d| d.into_scanned_device())
    }

    /// Run a scan job (async)
    pub async fn run_job(&self, job_id: Uuid) -> Result<()> {
        // Get job configuration
        let (targets, ports, timeout_ms, concurrency) = {
            let mut jobs = self.jobs.write().await;
            let job = jobs.get_mut(&job_id).ok_or_else(|| {
                Error::NotFound("Job not found".to_string())
            })?;
            job.status = JobStatus::Running;
            job.started_at = Some(chrono::Utc::now());
            job.current_phase = Some("Stage 0: 準備".to_string());
            job.progress_percent = Some(5);
            job.logs.push(ScanLogEntry::new("*", ScanLogEventType::Info, &format!("スキャン開始: targets={:?}", job.targets)));
            (
                job.targets.clone(),
                job.ports.clone(),
                job.timeout_ms,
                job.concurrency,
            )
        };

        tracing::info!(
            job_id = %job_id,
            targets = ?targets,
            "Starting scan job"
        );

        // Stage 0: Preparation - Parse CIDR targets and detect L2/L3
        let local_ip = get_local_ip().await.unwrap_or_else(|| "0.0.0.0".to_string());
        tracing::info!(job_id = %job_id, local_ip = %local_ip, "Detected local IP");
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("ローカルIP検出: {}", local_ip))).await;

        let mut all_ips: Vec<IpAddr> = Vec::new();
        let mut l2_targets: Vec<String> = Vec::new();
        let mut l3_targets: Vec<String> = Vec::new();

        for target in &targets {
            if is_local_subnet(target, &local_ip) {
                l2_targets.push(target.clone());
                tracing::info!(target = %target, "L2 subnet (ARP scan enabled)");
                self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("[L2] {} - ARPスキャン有効", target))).await;
            } else {
                l3_targets.push(target.clone());
                tracing::info!(target = %target, "L3 subnet (TCP scan only)");
                self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("[L3] {} - TCPスキャンのみ", target))).await;
            }

            match parse_cidr(target) {
                Ok(ips) => {
                    self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("CIDR解析: {} → {}ホスト", target, ips.len()))).await;
                    all_ips.extend(ips);
                }
                Err(e) => {
                    tracing::warn!(target = %target, error = %e, "Failed to parse CIDR");
                    self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Error, &format!("CIDR解析失敗: {} - {}", target, e))).await;
                }
            }
        }

        // Deduplicate
        all_ips.sort_by(|a, b| match (a, b) {
            (IpAddr::V4(a), IpAddr::V4(b)) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        });
        all_ips.dedup();

        let total_ips = all_ips.len() as u32;
        tracing::info!(
            job_id = %job_id,
            total_ips = total_ips,
            l2_subnets = l2_targets.len(),
            l3_subnets = l3_targets.len(),
            "Stage 0: Prep complete"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 0完了: 計{}ホスト (L2:{}, L3:{})", total_ips, l2_targets.len(), l3_targets.len()))).await;

        // Stage 1 & 2: Host Discovery + MAC/OUI
        // For L2 subnets: Use ARP scan (gets MAC addresses)
        // For L3 subnets: Use TCP port scan
        self.update_phase(&job_id, "Stage 1-2: ホスト検出", 10).await;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency as usize));

        // MAC address map from ARP scan (IP -> (MAC, Vendor))
        let mut arp_results: HashMap<IpAddr, (String, Option<String>)> = HashMap::new();
        let mut alive_hosts: Vec<IpAddr> = Vec::new();

        // L2 subnets: ARP scan
        for target in &l2_targets {
            tracing::info!(job_id = %job_id, target = %target, "Stage 1/2: ARP scan for L2 subnet");
            self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("ARPスキャン開始: {}", target))).await;
            let results = arp_scan_subnet(target, None).await;
            let result_count = results.len();
            for result in results {
                // 各ARP応答をログに記録
                let vendor_str = result.vendor.as_deref().unwrap_or("不明");
                self.add_log(&job_id, ScanLogEntry::new(&result.ip.to_string(), ScanLogEventType::ArpResponse,
                    &format!("ARP応答: MAC={} ベンダー={}", result.mac, vendor_str))
                    .with_oui(vendor_str)).await;

                arp_results.insert(result.ip, (result.mac.clone(), result.vendor.clone()));
                if !alive_hosts.contains(&result.ip) {
                    alive_hosts.push(result.ip);
                }
            }
            // Per-subnet feedback with network warning if no hosts found
            if result_count == 0 {
                tracing::warn!(job_id = %job_id, target = %target, "L2 subnet: No ARP responses - possible network issue");
                self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Warning,
                    &format!("⚠️ {} - ARP応答なし。ネットワーク設定を確認してください", target))).await;
            } else {
                self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("ARPスキャン完了: {} - {}ホスト発見", target, result_count))).await;
            }
        }

        // L3 subnets: TCP discovery (per-subnet tracking for better UX)
        if !l3_targets.is_empty() {
            tracing::info!(job_id = %job_id, "Stage 1: TCP discovery for L3 subnets");

            // Process each L3 subnet separately for better feedback
            for target in &l3_targets {
                self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("[L3] TCPスキャン開始: {}", target))).await;

                // Extract IPs belonging to this subnet
                let subnet_prefix = target.split('/').next().unwrap_or("");
                let prefix_parts: Vec<&str> = subnet_prefix.split('.').collect();

                let subnet_ips: Vec<IpAddr> = all_ips
                    .iter()
                    .filter(|ip| {
                        let ip_str = ip.to_string();
                        let ip_parts: Vec<&str> = ip_str.split('.').collect();
                        prefix_parts.len() >= 3
                            && ip_parts.len() >= 3
                            && prefix_parts[0] == ip_parts[0]
                            && prefix_parts[1] == ip_parts[1]
                            && prefix_parts[2] == ip_parts[2]
                    })
                    .cloned()
                    .collect();

                let mut discovery_handles = Vec::new();
                for ip in &subnet_ips {
                    let ip = *ip;
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    let handle = tokio::spawn(async move {
                        let result = discover_host(ip, timeout_ms).await;
                        drop(permit);
                        result
                    });
                    discovery_handles.push(handle);
                }

                let mut subnet_alive_count = 0;
                for handle in discovery_handles {
                    if let Ok(result) = handle.await {
                        if result.alive {
                            subnet_alive_count += 1;
                            if !alive_hosts.contains(&result.ip) {
                                alive_hosts.push(result.ip);
                            }
                        }
                    }
                }

                // Per-subnet feedback with network warning if no hosts found
                if subnet_alive_count == 0 {
                    tracing::warn!(job_id = %job_id, target = %target, "L3 subnet: No hosts found - possible network issue");
                    self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Warning,
                        &format!("⚠️ {} - ホスト未検出。ネットワーク到達性に問題がある可能性があります（ゲートウェイ含め応答なし）", target))).await;
                } else {
                    self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info,
                        &format!("[L3] TCPスキャン完了: {} - {}ホスト発見", target, subnet_alive_count))).await;
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            hosts_alive = alive_hosts.len(),
            arp_hosts = arp_results.len(),
            "Stage 1/2: Host discovery complete"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 1-2完了: {}ホスト発見 (ARP:{})", alive_hosts.len(), arp_results.len()))).await;

        // Stage 3: Port Scan (parallel)
        self.update_phase(&job_id, "Stage 3: ポートスキャン", 30).await;
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("ポートスキャン開始: {}ホスト, ポート={:?}", alive_hosts.len(), ports))).await;
        let mut port_results: HashMap<IpAddr, Vec<u16>> = HashMap::new();
        let mut port_handles = Vec::new();

        for ip in alive_hosts.clone() {
            let ports_clone = ports.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                let results = scan_ports(ip, &ports_clone, timeout_ms).await;
                drop(permit);
                (ip, results)
            });
            port_handles.push(handle);
        }

        for handle in port_handles {
            if let Ok((ip, results)) = handle.await {
                let open_ports: Vec<u16> = results
                    .iter()
                    .filter(|r| r.open)
                    .map(|r| r.port)
                    .collect();
                if !open_ports.is_empty() {
                    // 各オープンポートをログに記録
                    for port in &open_ports {
                        self.add_log(&job_id, ScanLogEntry::new(&ip.to_string(), ScanLogEventType::PortOpen,
                            &format!("ポート{} OPEN", port)).with_port(*port)).await;
                    }
                    port_results.insert(ip, open_ports);
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            hosts_with_ports = port_results.len(),
            "Stage 3: Port scan complete"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 3完了: {}ホストにオープンポートあり", port_results.len()))).await;

        // Stage 4: Discovery Probes (ONVIF, RTSP) + MAC/OUI from ARP
        self.update_phase(&job_id, "Stage 4: プロトコル検査", 50).await;
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("ONVIF/RTSPプローブ開始: {}ホスト", port_results.len()))).await;
        let mut evidence_map: HashMap<IpAddr, DeviceEvidence> = HashMap::new();
        let mut probe_handles = Vec::new();

        // Clone ARP results for use in spawned tasks
        let arp_results_arc = Arc::new(arp_results.clone());

        for (ip, open_ports) in &port_results {
            let ip = *ip;
            let open_ports_clone = open_ports.clone();
            let arp_data = arp_results_arc.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                // ONVIF probe with detailed result
                let (onvif_found, onvif_result) = probe_onvif_detailed(ip, timeout_ms).await;

                // RTSP probe on 554 and 8554 with detailed result
                let mut rtsp_found = false;
                let mut rtsp_result = ProbeResult::NotTested;
                for &port in &[554, 8554] {
                    if open_ports_clone.contains(&port) {
                        let (found, result) = probe_rtsp_detailed(ip, port, timeout_ms).await;
                        if found {
                            rtsp_found = true;
                            rtsp_result = result;
                            break;
                        }
                        // 認証エラーでもカメラの可能性あり
                        if matches!(result, ProbeResult::AuthRequired) {
                            rtsp_result = result;
                        } else if !matches!(rtsp_result, ProbeResult::AuthRequired) {
                            rtsp_result = result;
                        }
                    }
                }

                drop(permit);

                // Get MAC/OUI from ARP results (L2 only)
                let (mac, oui_vendor) = if let Some((mac_addr, vendor)) = arp_data.get(&ip) {
                    (Some(mac_addr.clone()), vendor.clone())
                } else {
                    (None, None)
                };

                DeviceEvidence {
                    ip,
                    open_ports: open_ports_clone,
                    mac,
                    oui_vendor,
                    onvif_found,
                    ssdp_found: false,
                    mdns_found: false,
                    onvif_result,
                    rtsp_result,
                }
            });
            probe_handles.push(handle);
        }

        for handle in probe_handles {
            if let Ok(evidence) = handle.await {
                // ONVIF/RTSPプローブ結果をログ
                let onvif_str = match evidence.onvif_result {
                    ProbeResult::Success => "ONVIF成功",
                    ProbeResult::AuthRequired => "ONVIF認証必要",
                    ProbeResult::Timeout => "ONVIFタイムアウト",
                    ProbeResult::Refused => "ONVIF拒否",
                    ProbeResult::NoResponse => "ONVIF応答なし",
                    ProbeResult::Error => "ONVIFエラー",
                    ProbeResult::NotTested => "ONVIF未テスト",
                };
                let rtsp_str = match evidence.rtsp_result {
                    ProbeResult::Success => "RTSP成功",
                    ProbeResult::AuthRequired => "RTSP認証必要",
                    ProbeResult::Timeout => "RTSPタイムアウト",
                    ProbeResult::Refused => "RTSP拒否",
                    ProbeResult::NoResponse => "RTSP応答なし",
                    ProbeResult::Error => "RTSPエラー",
                    ProbeResult::NotTested => "RTSP未テスト",
                };
                self.add_log(&job_id, ScanLogEntry::new(&evidence.ip.to_string(), ScanLogEventType::OnvifProbe,
                    &format!("{}, {}", onvif_str, rtsp_str))).await;

                evidence_map.insert(evidence.ip, evidence);
            }
        }

        tracing::info!(
            job_id = %job_id,
            probed_hosts = evidence_map.len(),
            "Stage 4: Discovery probes complete"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 4完了: {}ホストをプローブ", evidence_map.len()))).await;

        // Stage 5: Scoring (全デバイス保存、閾値フィルタなし)
        // カメラ関連ポートが開いているデバイスは全て保存
        self.update_phase(&job_id, "Stage 5: スコアリング", 60).await;
        let camera_ports: &[u16] = &[554, 2020, 8554, 80, 443, 8000, 8080, 8443];
        let mut candidates: Vec<(IpAddr, u32, DeviceEvidence)> = Vec::new();

        for (ip, evidence) in evidence_map {
            let score = calculate_score(&evidence);
            // カメラ関連ポートが1つでも開いていれば保存
            let has_camera_port = evidence.open_ports.iter().any(|p| camera_ports.contains(p));
            if has_camera_port {
                self.add_log(&job_id, ScanLogEntry::new(&ip.to_string(), ScanLogEventType::DeviceClassified,
                    &format!("スコア={} ports={:?}", score, evidence.open_ports))).await;
                candidates.push((ip, score, evidence));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by score descending

        tracing::info!(
            job_id = %job_id,
            candidates = candidates.len(),
            "Stage 5: Scoring complete (no threshold filter, all camera-port devices saved)"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 5完了: {}台がカメラ候補", candidates.len()))).await;

        // Stage 6: Verification - Store results to DB (without credential verification)
        // Note: Credential verification requires admin input via separate API
        self.update_phase(&job_id, "Stage 6: DB保存", 70).await;
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("デバイス情報をDBに保存中..."))).await;
        let now = chrono::Utc::now();
        let mut cameras_found = 0u32;

        for (ip, score, evidence) in candidates {
            let device_id = Uuid::new_v4();
            let ip_str = ip.to_string();

            // Generate DetectionReason for user feedback
            let detection = generate_detection_reason(&evidence);

            // Determine camera family from evidence (OUI + ONVIF + ports)
            let family = determine_camera_family(&evidence);

            let family_str = match family {
                CameraFamily::Tapo => "tapo",
                CameraFamily::Vigi => "vigi",
                CameraFamily::Nest => "nest",
                CameraFamily::Axis => "axis",
                CameraFamily::Hikvision => "hikvision",
                CameraFamily::Dahua => "dahua",
                CameraFamily::Other => "other",
                CameraFamily::Unknown => "unknown",
            };

            let open_ports_json = serde_json::to_string(&evidence.open_ports).unwrap_or_else(|_| "[]".to_string());
            let detection_json = serde_json::to_string(&detection).unwrap_or_else(|_| "{}".to_string());
            let subnet = extract_subnet(&ip_str);
            let confidence = calculate_confidence(score);

            // Log device detection with user message
            tracing::info!(
                ip = %ip_str,
                mac = ?evidence.mac,
                oui = ?evidence.oui_vendor,
                family = %family_str,
                device_type = ?detection.device_type,
                user_message = %detection.user_message,
                suggested_action = ?detection.suggested_action,
                "Device detected"
            );

            // Upsert to DB with detection_json
            let result = sqlx::query(
                "INSERT INTO ipcamscan_devices \
                 (device_id, ip, subnet, mac, oui_vendor, open_ports, score, verified, status, family, confidence, detection_json, first_seen, last_seen) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, 'discovered', ?, ?, ?, ?, ?) \
                 ON DUPLICATE KEY UPDATE \
                 mac = COALESCE(VALUES(mac), mac), \
                 oui_vendor = COALESCE(VALUES(oui_vendor), oui_vendor), \
                 open_ports = VALUES(open_ports), \
                 score = VALUES(score), \
                 family = VALUES(family), \
                 confidence = VALUES(confidence), \
                 detection_json = VALUES(detection_json), \
                 last_seen = VALUES(last_seen)"
            )
            .bind(device_id.to_string())
            .bind(&ip_str)
            .bind(&subnet)
            .bind(&evidence.mac)
            .bind(&evidence.oui_vendor)
            .bind(&open_ports_json)
            .bind(score)
            .bind(family_str)
            .bind(confidence)
            .bind(&detection_json)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await;

            match result {
                Ok(_) => cameras_found += 1,
                Err(e) => {
                    tracing::error!(ip = %ip_str, error = %e, "Failed to save device to DB");
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            cameras_found = cameras_found,
            "Stage 6: Results stored"
        );
        self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 6完了: {}台をDBに保存", cameras_found))).await;

        // Stage 7: Automatic credential trial
        // Fetch credentials for each subnet and try on camera-like devices
        self.update_phase(&job_id, "Stage 7: 認証試行", 80).await;
        let mut cameras_verified = 0u32;

        // Get unique subnets from targets
        let mut subnet_credentials: HashMap<String, Vec<TrialCredential>> = HashMap::new();
        for target in &targets {
            // Fetch credentials for this subnet
            let creds_result: std::result::Result<Option<(Option<String>,)>, _> = sqlx::query_as(
                "SELECT credentials FROM scan_subnets WHERE cidr = ?"
            )
            .bind(target)
            .fetch_optional(&self.pool)
            .await;

            if let Ok(Some((Some(creds_json),))) = creds_result {
                if let Ok(creds) = serde_json::from_str::<Vec<TrialCredential>>(&creds_json) {
                    if !creds.is_empty() {
                        // Sort by priority
                        let mut sorted_creds = creds;
                        sorted_creds.sort_by_key(|c| c.priority);
                        let cred_count = sorted_creds.len();
                        subnet_credentials.insert(target.clone(), sorted_creds);
                        tracing::info!(subnet = %target, count = cred_count, "Loaded subnet credentials");
                    }
                }
            }
        }

        if !subnet_credentials.is_empty() {
            tracing::info!(
                job_id = %job_id,
                subnets_with_creds = subnet_credentials.len(),
                "Stage 7: Starting credential trial"
            );
            self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("クレデンシャル試行開始: {}サブネットに設定あり", subnet_credentials.len()))).await;

            // Get all camera-like devices we just saved
            let devices = self.list_devices(DeviceFilter::default()).await;
            self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("認証対象: {}台のカメラ候補", devices.len()))).await;

            for device in devices {
                // Only try credentials on devices that look like cameras
                let should_try = match device.detection.device_type {
                    DeviceType::CameraConfirmed | DeviceType::CameraLikely | DeviceType::CameraPossible => true,
                    _ => false,
                };

                if !should_try {
                    continue;
                }

                // Find credentials for this device's subnet
                // Extract network prefix from device subnet (e.g., "192.168.96" from "192.168.96.0/24")
                let device_ip = &device.ip;
                let creds = subnet_credentials.iter()
                    .find(|(subnet_cidr, _)| {
                        // Check if device IP falls within the subnet CIDR
                        ip_in_cidr(device_ip, subnet_cidr)
                    })
                    .map(|(_, c)| c);

                if let Some(credentials) = creds {
                    let ip: IpAddr = match device.ip.parse() {
                        Ok(ip) => ip,
                        Err(_) => continue,
                    };

                    self.add_log(&job_id, ScanLogEntry::new(&device.ip, ScanLogEventType::CredentialTrial,
                        &format!("認証試行開始: {}個のクレデンシャル", credentials.len()))).await;

                    let mut success = false;
                    let mut success_username: Option<String> = None;
                    let mut success_password: Option<String> = None;
                    let mut auto_completed_label: Option<String> = None;
                    let mut device_info: Option<OnvifDeviceInfo> = None;

                    // Check if device is a confirmed camera (ONVIF or RTSP responded)
                    // Password variations should always be tried for camera_confirmed devices
                    // because Tapo cameras respond to unauthenticated ONVIF GetDeviceInformation
                    // but still require auth for GetCapabilities and actual usage
                    let is_camera_confirmed = matches!(
                        device.detection.device_type,
                        DeviceType::CameraConfirmed
                    );

                    // Try each credential in priority order
                    'cred_loop: for cred in credentials {
                        tracing::debug!(
                            ip = %device.ip,
                            username = %cred.username,
                            priority = cred.priority,
                            "Trying credential"
                        );

                        // Generate password variations for camera_confirmed devices
                        // This covers Tapo cameras that respond without auth but need auth for full access
                        let password_variations = if is_camera_confirmed {
                            generate_password_variations(&cred.password)
                        } else {
                            // Only try original password for non-confirmed cameras
                            vec![(cred.password.clone(), "original".to_string())]
                        };

                        for (pass_variant, variant_desc) in &password_variations {
                            // Try ONVIF with auth first
                            if let Some(info) = probe_onvif_with_auth(ip, &cred.username, pass_variant, 5000).await {
                                success = true;
                                success_username = Some(cred.username.clone());
                                success_password = Some(pass_variant.clone());
                                device_info = Some(info);

                                if variant_desc != "original" {
                                    // Password variation worked - log with pass{} label
                                    auto_completed_label = Some(format!("pass{{{}}}", pass_variant));
                                    tracing::info!(
                                        ip = %device.ip,
                                        username = %cred.username,
                                        original_password = %cred.password,
                                        working_password = %pass_variant,
                                        variation = %variant_desc,
                                        label = %auto_completed_label.as_ref().unwrap(),
                                        "Auto-completed password success via ONVIF"
                                    );
                                } else {
                                    tracing::info!(
                                        ip = %device.ip,
                                        username = %cred.username,
                                        "Credential success via ONVIF"
                                    );
                                }
                                break 'cred_loop;
                            }

                            // Try RTSP as fallback (only for original password to reduce load)
                            if variant_desc == "original" {
                                if let Some(_rtsp_uri) = scanner::verify_rtsp(ip, 554, &cred.username, pass_variant, 5000).await {
                                    success = true;
                                    success_username = Some(cred.username.clone());
                                    success_password = Some(pass_variant.clone());
                                    tracing::info!(
                                        ip = %device.ip,
                                        username = %cred.username,
                                        "Credential success via RTSP"
                                    );
                                    break 'cred_loop;
                                }
                            }
                        }
                    }

                    // Update device with credential trial result
                    let (cred_status, manufacturer, model, firmware, mac, updated_detection) = if success {
                        cameras_verified += 1;
                        self.add_log(&job_id, ScanLogEntry::new(&device.ip, ScanLogEventType::CredentialTrial,
                            &format!("★認証成功: user={}", success_username.as_deref().unwrap_or("?")))).await;

                        // Update detection reason to reflect successful authentication
                        let mut new_detection = device.detection.clone();
                        new_detection.device_type = DeviceType::CameraConfirmed;
                        new_detection.suggested_action = SuggestedAction::None;
                        new_detection.user_message = "カメラ確認済み (認証成功)".to_string();

                        if let Some(info) = device_info {
                            let mfr = info.manufacturer.clone().unwrap_or_default();
                            let mdl = info.model.clone().unwrap_or_default();
                            let fw = info.firmware_version.clone().unwrap_or_default();
                            self.add_log(&job_id, ScanLogEntry::new(&device.ip, ScanLogEventType::Info,
                                &format!("デバイス情報: {} {} ({})", mfr, mdl, fw))).await;
                            ("success", info.manufacturer, info.model, info.firmware_version, info.mac_address, Some(new_detection))
                        } else {
                            ("success", None, None, None, None, Some(new_detection))
                        }
                    } else {
                        self.add_log(&job_id, ScanLogEntry::new(&device.ip, ScanLogEventType::CredentialTrial,
                            &format!("認証失敗: 全クレデンシャル不一致"))).await;
                        ("failed", None, None, None, None, None)
                    };

                    // Update status to 'verified' when credentials succeed
                    let new_status = if success { "verified" } else { "discovered" };
                    let detection_json = updated_detection
                        .map(|d| serde_json::to_string(&d).unwrap_or_else(|_| "{}".to_string()));

                    let _ = sqlx::query(
                        "UPDATE ipcamscan_devices SET \
                         credential_status = ?, \
                         credential_username = ?, \
                         credential_password = ?, \
                         manufacturer = COALESCE(?, manufacturer), \
                         model = COALESCE(?, model), \
                         firmware = COALESCE(?, firmware), \
                         mac = COALESCE(?, mac), \
                         verified = ?, \
                         status = ?, \
                         detection_json = COALESCE(?, detection_json) \
                         WHERE ip = ?"
                    )
                    .bind(cred_status)
                    .bind(&success_username)
                    .bind(&success_password)
                    .bind(&manufacturer)
                    .bind(&model)
                    .bind(&firmware)
                    .bind(&mac)
                    .bind(success)
                    .bind(new_status)
                    .bind(&detection_json)
                    .bind(&device.ip)
                    .execute(&self.pool)
                    .await;
                }
            }

            tracing::info!(
                job_id = %job_id,
                cameras_verified = cameras_verified,
                "Stage 7: Credential trial complete"
            );
            self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, &format!("Stage 7完了: {}台の認証成功", cameras_verified))).await;
        } else {
            self.add_log(&job_id, ScanLogEntry::new("*", ScanLogEventType::Info, "Stage 7スキップ: クレデンシャル未設定")).await;
        }

        // Update job summary
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Success;
                job.ended_at = Some(chrono::Utc::now());
                job.current_phase = Some("完了".to_string());
                job.progress_percent = Some(100);
                job.summary = Some(JobSummary {
                    total_ips,
                    hosts_alive: alive_hosts.len() as u32,
                    cameras_found,
                    cameras_verified,
                });
                job.logs.push(ScanLogEntry::new("*", ScanLogEventType::Info,
                    &format!("★スキャン完了: 発見={} 認証成功={}", cameras_found, cameras_verified)));
            }
        }

        tracing::info!(
            job_id = %job_id,
            total_ips = total_ips,
            hosts_alive = alive_hosts.len(),
            cameras_found = cameras_found,
            cameras_verified = cameras_verified,
            "Scan job completed"
        );

        Ok(())
    }

    /// Verify a device with credentials (Stage 6 completion)
    /// Also retrieves device info (manufacturer, model, MAC) via ONVIF
    pub async fn verify_device(
        &self,
        device_ip: &str,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let ip: IpAddr = device_ip
            .parse()
            .map_err(|_| Error::Validation("Invalid IP address".to_string()))?;

        // Update status to verifying
        let _ = sqlx::query(
            "UPDATE ipcamscan_devices SET status = 'verifying' WHERE ip = ?"
        )
        .bind(device_ip)
        .execute(&self.pool)
        .await;

        // Try RTSP verification
        let rtsp_uri = scanner::verify_rtsp(ip, 554, username, password, 5000).await;

        if let Some(uri) = rtsp_uri {
            // Also try to get device info via ONVIF with authentication
            let device_info = probe_onvif_with_auth(ip, username, password, 5000).await;

            // Build the update query with device info if available
            let (manufacturer, model, firmware, mac, oui_vendor) = if let Some(info) = device_info {
                tracing::info!(
                    ip = %device_ip,
                    manufacturer = ?info.manufacturer,
                    model = ?info.model,
                    mac = ?info.mac_address,
                    "Retrieved ONVIF device info"
                );

                let oui = info.mac_address.as_ref().and_then(|m| scanner::lookup_oui(m));

                (
                    info.manufacturer,
                    info.model,
                    info.firmware_version,
                    info.mac_address,
                    oui,
                )
            } else {
                (None, None, None, None, None)
            };

            // Update DB with verification success and device info
            let result = sqlx::query(
                "UPDATE ipcamscan_devices SET \
                 verified = 1, \
                 status = 'verified', \
                 rtsp_uri = ?, \
                 manufacturer = COALESCE(?, manufacturer), \
                 model = COALESCE(?, model), \
                 firmware = COALESCE(?, firmware), \
                 mac = COALESCE(?, mac), \
                 oui_vendor = COALESCE(?, oui_vendor), \
                 confidence = 95, \
                 last_seen = NOW(3) \
                 WHERE ip = ?"
            )
            .bind(&uri)
            .bind(&manufacturer)
            .bind(&model)
            .bind(&firmware)
            .bind(&mac)
            .bind(&oui_vendor)
            .bind(device_ip)
            .execute(&self.pool)
            .await;

            match result {
                Ok(_) => return Ok(true),
                Err(e) => {
                    tracing::error!(ip = %device_ip, error = %e, "Failed to update verified device");
                    return Err(Error::Internal(format!("Database error: {}", e)));
                }
            }
        } else {
            // Update status back to discovered (verification failed)
            let _ = sqlx::query(
                "UPDATE ipcamscan_devices SET status = 'discovered' WHERE ip = ?"
            )
            .bind(device_ip)
            .execute(&self.pool)
            .await;
        }

        Ok(false)
    }

    /// Reject a device (admin action)
    pub async fn reject_device(&self, device_ip: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE ipcamscan_devices SET status = 'rejected', last_seen = NOW(3) WHERE ip = ?"
        )
        .bind(device_ip)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => Ok(()),
            Ok(_) => Err(Error::NotFound(format!("Device not found: {}", device_ip))),
            Err(e) => Err(Error::Internal(format!("Database error: {}", e))),
        }
    }

    /// Delete a single scanned device by IP
    pub async fn delete_device(&self, device_ip: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM ipcamscan_devices WHERE ip = ?")
            .bind(device_ip)
            .execute(&self.pool)
            .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => Ok(()),
            Ok(_) => Err(Error::NotFound(format!("Device not found: {}", device_ip))),
            Err(e) => Err(Error::Internal(format!("Database error: {}", e))),
        }
    }

    /// Delete all scanned devices
    pub async fn clear_devices(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM ipcamscan_devices")
            .execute(&self.pool)
            .await;

        match result {
            Ok(r) => Ok(r.rows_affected()),
            Err(e) => Err(Error::Internal(format!("Database error: {}", e))),
        }
    }

    /// Approve a verified device and create camera entry
    pub async fn approve_device(
        &self,
        device_ip: &str,
        request: &ApproveDeviceRequest,
    ) -> Result<ApproveDeviceResponse> {
        // Get device from DB
        let device = self.get_device(device_ip).await
            .ok_or_else(|| Error::NotFound(format!("Device not found: {}", device_ip)))?;

        // Must be verified before approval
        if device.status != DeviceStatus::Verified {
            return Err(Error::Validation(
                "Device must be verified before approval. Current status: {}".to_string()
            ));
        }

        // Check for duplicate by IP address
        let existing_by_ip: Option<(String,)> = sqlx::query_as(
            "SELECT camera_id FROM cameras WHERE ip_address = ?"
        )
        .bind(device_ip)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Database error: {}", e)))?;

        if let Some((existing_id,)) = existing_by_ip {
            return Err(Error::Validation(format!(
                "Camera with IP {} already exists: {}",
                device_ip, existing_id
            )));
        }

        // Check for duplicate by MAC address (if available)
        if let Some(ref mac) = device.mac {
            let existing_by_mac: Option<(String, String)> = sqlx::query_as(
                "SELECT camera_id, ip_address FROM cameras WHERE mac_address = ?"
            )
            .bind(mac)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::Internal(format!("Database error: {}", e)))?;

            if let Some((existing_id, existing_ip)) = existing_by_mac {
                return Err(Error::Validation(format!(
                    "Camera with MAC {} already exists: {} (IP: {})",
                    mac, existing_id, existing_ip
                )));
            }
        }

        // Generate lacisID
        // Format: [Prefix=3][ProductType=022][MAC or IP hash=12桁][ProductCode=0001]
        let mac_part = device.mac
            .as_ref()
            .map(|m| m.replace(":", "").replace("-", "").to_uppercase())
            .unwrap_or_else(|| {
                // Use IP hash if MAC not available
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                device_ip.hash(&mut hasher);
                format!("{:012X}", hasher.finish() & 0xFFFFFFFFFFFF)
            });

        let lacis_id = format!("3022{}0001", &mac_part[..12.min(mac_part.len())].to_uppercase());

        // Generate camera_id
        let camera_id = format!("cam-{}", Uuid::new_v4());

        // Determine credentials: request.credentials > bound credentials (from scan) > fallback
        let (use_username, use_password): (Option<String>, Option<String>) = if let Some(ref creds) = request.credentials {
            // Explicit credentials provided in request
            (Some(creds.username.clone()), Some(creds.password.clone()))
        } else if device.credential_username.is_some() && device.credential_password.is_some() {
            // Use bound credentials from scan (credential trial)
            tracing::info!(
                ip = %device_ip,
                username = ?device.credential_username,
                "Using bound credentials from scan"
            );
            (device.credential_username.clone(), device.credential_password.clone())
        } else {
            // No credentials available
            (None, None)
        };

        // Build RTSP URL with credentials if available
        let rtsp_url = if let (Some(ref user), Some(ref pass)) = (&use_username, &use_password) {
            // URL-encode the password (@ becomes %40)
            let encoded_pass = pass.replace("@", "%40");
            Some(format!(
                "rtsp://{}:{}@{}:554/stream1",
                user, encoded_pass, device_ip
            ))
        } else {
            // Fall back to stored rtsp_uri (may not have credentials)
            device.rtsp_uri.clone()
        };

        // Insert into cameras table with credentials
        let result = sqlx::query(
            "INSERT INTO cameras \
             (camera_id, name, location, ip_address, mac_address, rtsp_main, rtsp_username, rtsp_password, family, \
              source_device_id, fid, lacis_id, enabled, polling_enabled) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 1)"
        )
        .bind(&camera_id)
        .bind(&request.display_name)
        .bind(&request.location)
        .bind(device_ip)
        .bind(&device.mac)
        .bind(&rtsp_url)
        .bind(&use_username)
        .bind(&use_password)
        .bind(match device.family {
            CameraFamily::Tapo => "tapo",
            CameraFamily::Vigi => "vigi",
            CameraFamily::Nest => "nest",
            CameraFamily::Axis => "axis",
            CameraFamily::Hikvision => "hikvision",
            CameraFamily::Dahua => "dahua",
            CameraFamily::Other => "other",
            CameraFamily::Unknown => "unknown",
        })
        .bind(device.device_id.to_string())
        .bind(&request.fid)
        .bind(&lacis_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                // Update device status to approved
                let _ = sqlx::query(
                    "UPDATE ipcamscan_devices SET status = 'approved', last_seen = NOW(3) WHERE ip = ?"
                )
                .bind(device_ip)
                .execute(&self.pool)
                .await;

                // Log registration
                let _ = sqlx::query(
                    "INSERT INTO camera_registration_log (device_id, camera_id, action, performed_by) \
                     VALUES (?, ?, 'approved', 'admin')"
                )
                .bind(device.device_id.to_string())
                .bind(&camera_id)
                .execute(&self.pool)
                .await;

                Ok(ApproveDeviceResponse {
                    camera_id,
                    lacis_id,
                    ip_address: device_ip.to_string(),
                    rtsp_url,
                })
            }
            Err(e) => Err(Error::Internal(format!("Database error: {}", e))),
        }
    }

    /// Batch verify devices using facility credentials
    /// Note: This is a stub - proper AES decryption will be implemented later
    pub async fn batch_verify(
        &self,
        fid: &str,
        _device_ids: &[Uuid],
    ) -> Result<Vec<BatchVerifyResult>> {
        // Get facility credentials
        let creds: Option<(String, Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT username, password_encrypted, encryption_iv FROM facility_credentials WHERE fid = ?"
        )
        .bind(fid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Internal(format!("Database error: {}", e)))?;

        match creds {
            Some((_username, _password_encrypted, _iv)) => {
                // TODO: Implement proper AES-256 decryption
                // For now, we'll return an error
                Err(Error::Validation(
                    "Facility credentials decryption not yet implemented. Use individual verify API.".to_string()
                ))
            }
            None => {
                Err(Error::NotFound(format!(
                    "Facility credentials not found for fid: {}",
                    fid
                )))
            }
        }
    }
}

/// Database row for scanned device
#[derive(Debug, sqlx::FromRow)]
struct DbDevice {
    device_id: String,
    ip: String,
    subnet: String,
    mac: Option<String>,
    oui_vendor: Option<String>,
    hostnames: Option<String>,     // JSON
    open_ports: String,            // JSON
    score: u32,
    verified: bool,
    status: String,
    manufacturer: Option<String>,
    model: Option<String>,
    firmware: Option<String>,
    family: String,
    confidence: u8,
    rtsp_uri: Option<String>,
    detection_json: Option<String>, // JSON - DetectionReason
    first_seen: chrono::DateTime<chrono::Utc>,
    last_seen: chrono::DateTime<chrono::Utc>,
    credential_status: Option<String>,    // クレデンシャル試行結果
    credential_username: Option<String>,  // 成功したユーザー名
    credential_password: Option<String>,  // 成功したパスワード
}

impl DbDevice {
    fn into_scanned_device(self) -> ScannedDevice {
        let open_ports: Vec<u16> = serde_json::from_str(&self.open_ports).unwrap_or_default();

        // Parse detection_json if present
        let detection: DetectionReason = self.detection_json
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

        ScannedDevice {
            device_id: Uuid::parse_str(&self.device_id).unwrap_or_else(|_| Uuid::nil()),
            ip: self.ip,
            subnet: self.subnet,
            mac: self.mac,
            oui_vendor: self.oui_vendor,
            hostnames: self.hostnames
                .and_then(|h| serde_json::from_str(&h).ok())
                .unwrap_or_default(),
            open_ports: open_ports
                .iter()
                .map(|&p| PortStatus {
                    port: p,
                    status: PortState::Open,
                })
                .collect(),
            score: self.score,
            verified: self.verified,
            status: match self.status.as_str() {
                "discovered" => DeviceStatus::Discovered,
                "verifying" => DeviceStatus::Verifying,
                "verified" => DeviceStatus::Verified,
                "rejected" => DeviceStatus::Rejected,
                "approved" => DeviceStatus::Approved,
                _ => DeviceStatus::Discovered,
            },
            manufacturer: self.manufacturer,
            model: self.model,
            firmware: self.firmware,
            family: match self.family.as_str() {
                "tapo" => CameraFamily::Tapo,
                "vigi" => CameraFamily::Vigi,
                "nest" => CameraFamily::Nest,
                "axis" => CameraFamily::Axis,
                "hikvision" => CameraFamily::Hikvision,
                "dahua" => CameraFamily::Dahua,
                "other" => CameraFamily::Other,
                _ => CameraFamily::Unknown,
            },
            confidence: self.confidence,
            rtsp_uri: self.rtsp_uri,
            first_seen: self.first_seen,
            last_seen: self.last_seen,
            detection,
            credential_status: match self.credential_status.as_deref() {
                Some("success") => CredentialStatus::Success,
                Some("failed") => CredentialStatus::Failed,
                _ => CredentialStatus::NotTried,
            },
            credential_username: self.credential_username,
            credential_password: self.credential_password,
        }
    }
}

// Note: IpcamScan no longer implements Default as it requires a DB pool
// Use IpcamScan::new(pool) instead

fn default_ports() -> Vec<u16> {
    vec![554, 2020, 80, 443, 8000, 8080, 8443, 8554]
}

fn extract_subnet(ip: &str) -> String {
    // Extract /24 subnet
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.{}.0/24", parts[0], parts[1], parts[2])
    } else {
        ip.to_string()
    }
}

/// Check if an IP address falls within a CIDR subnet
/// e.g., ip_in_cidr("192.168.96.83", "192.168.96.0/23") = true
fn ip_in_cidr(ip: &str, cidr: &str) -> bool {
    use std::net::Ipv4Addr;

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

fn calculate_confidence(score: u32) -> u8 {
    // Map score to confidence (0-100)
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
fn determine_camera_family(evidence: &scanner::DeviceEvidence) -> CameraFamily {
    if let Some(ref vendor) = evidence.oui_vendor {
        let vendor_upper = vendor.to_uppercase();
        if vendor_upper.contains("TP-LINK") || vendor_upper.contains("TPLINK") {
            if evidence.onvif_found && evidence.open_ports.contains(&2020) {
                CameraFamily::Tapo
            } else if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&2020) {
                CameraFamily::Tapo // TP-Link with camera ports
            } else {
                CameraFamily::Other // TP-Link but possibly not camera
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
        CameraFamily::Tapo // Assume Tapo-like
    } else {
        CameraFamily::Unknown
    }
}

/// Generate user-friendly DetectionReason from device evidence
fn generate_detection_reason(evidence: &scanner::DeviceEvidence) -> DetectionReason {
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
    let camera_ports: Vec<u16> = evidence.open_ports.iter()
        .filter(|p| camera_related_ports.contains(p))
        .cloned()
        .collect();

    // Determine device type and user message
    let (device_type, user_message, suggested_action) = if evidence.onvif_found &&
        (matches!(evidence.rtsp_result, ProbeResult::Success) || evidence.open_ports.contains(&554)) {
        // ONVIF成功 + RTSP応答あり = カメラ確定
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (ONVIF/RTSP応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if evidence.onvif_found {
        // ONVIF成功のみ
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (ONVIF応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if matches!(evidence.rtsp_result, ProbeResult::Success) {
        // RTSP成功
        (
            DeviceType::CameraConfirmed,
            "カメラ確認済み (RTSP応答あり)".to_string(),
            SuggestedAction::None,
        )
    } else if matches!(evidence.onvif_result, ProbeResult::AuthRequired) ||
              matches!(evidence.rtsp_result, ProbeResult::AuthRequired) {
        // 認証エラー = カメラの可能性高
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
        // OUIベンダー一致 + カメラポート開
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
        // RTSPポート開いているがプローブ失敗
        (
            DeviceType::CameraPossible,
            "RTSPポート検出 (カメラの可能性あり、クレデンシャル設定が必要)".to_string(),
            SuggestedAction::SetCredentials,
        )
    } else if evidence.open_ports.contains(&2020) {
        // ONVIFポート開いているがプローブ失敗
        (
            DeviceType::CameraPossible,
            "ONVIFポート検出 (カメラの可能性あり、クレデンシャル設定が必要)".to_string(),
            SuggestedAction::SetCredentials,
        )
    } else if evidence.open_ports.contains(&8000) && evidence.open_ports.contains(&8080) {
        // NVR特有のポートパターン
        (
            DeviceType::NvrLikely,
            "NVR/録画機器の可能性あり".to_string(),
            SuggestedAction::ManualCheck,
        )
    } else if evidence.open_ports.contains(&443) || evidence.open_ports.contains(&8443) {
        // HTTPS管理ポートのみ
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
/// 1. 先頭文字の大文字/小文字切り替え
/// 2. 末尾@の追加/削除
/// Returns: Vec<(password_variation, variation_description)>
fn generate_password_variations(password: &str) -> Vec<(String, String)> {
    let mut variations = Vec::new();

    // Helper: toggle first character case
    let toggle_first_char = |s: &str| -> Option<String> {
        let mut chars: Vec<char> = s.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let first = chars[0];
        if first.is_alphabetic() {
            chars[0] = if first.is_uppercase() {
                first.to_lowercase().next().unwrap_or(first)
            } else {
                first.to_uppercase().next().unwrap_or(first)
            };
            let toggled: String = chars.into_iter().collect();
            if toggled != s {
                return Some(toggled);
            }
        }
        None
    };

    // 1. Original password
    variations.push((password.to_string(), "original".to_string()));

    // 2. First char toggled
    if let Some(toggled) = toggle_first_char(password) {
        variations.push((toggled, "first_char_toggled".to_string()));
    }

    if password.ends_with('@') {
        // Password ends with @ - try removing it
        let without_at = password.trim_end_matches('@').to_string();
        if !without_at.is_empty() && without_at != password {
            variations.push((without_at.clone(), "without_trailing_at".to_string()));

            // Also try with first char toggled
            if let Some(toggled) = toggle_first_char(&without_at) {
                variations.push((toggled, "without_at+first_toggled".to_string()));
            }
        }
    } else {
        // Password doesn't end with @ - try adding @, @@, @@@
        for (suffix, suffix_name) in [("@", "at1"), ("@@", "at2"), ("@@@", "at3")] {
            let with_suffix = format!("{}{}", password, suffix);
            variations.push((with_suffix.clone(), format!("with_{}", suffix_name)));

            // Also try with first char toggled
            if let Some(toggled) = toggle_first_char(&with_suffix) {
                variations.push((toggled, format!("with_{}+first_toggled", suffix_name)));
            }
        }
    }

    variations
}
