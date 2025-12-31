//! IpcamScan - Camera Auto-Discovery
//!
//! Issue #30 [IS22-GAP-004] IpcamScan implementation
//!
//! ## Responsibilities
//!
//! - Multi-stage evidence-based discovery
//! - Stage 0-6: Prep → Host → MAC → Port → Discovery → Score → Verify
//! - No brute-force, proper credentials only

mod scanner;
mod types;

pub use types::*;

use crate::error::{Error, Result};
use scanner::{
    calculate_score, discover_host, parse_cidr, probe_onvif, probe_rtsp, scan_ports,
    DeviceEvidence, SCORE_THRESHOLD_VERIFY,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// IpcamScan service
pub struct IpcamScan {
    jobs: Arc<RwLock<HashMap<Uuid, ScanJob>>>,
    devices: Arc<RwLock<HashMap<String, ScannedDevice>>>,
}

impl IpcamScan {
    /// Create new IpcamScan
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new scan job
    pub async fn create_job(&self, request: ScanJobRequest) -> ScanJob {
        let job = ScanJob {
            job_id: Uuid::new_v4(),
            targets: request.targets,
            mode: request.mode.unwrap_or_default(),
            ports: request.ports.unwrap_or_else(default_ports),
            timeout_ms: request.timeout_ms.unwrap_or(500),
            concurrency: request.concurrency.unwrap_or(10),
            status: JobStatus::Queued,
            started_at: None,
            ended_at: None,
            summary: None,
            created_at: chrono::Utc::now(),
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job.job_id, job.clone());
        }

        tracing::info!(job_id = %job.job_id, "Scan job created");

        job
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

    /// Get discovered devices
    pub async fn list_devices(&self, filter: DeviceFilter) -> Vec<ScannedDevice> {
        let devices = self.devices.read().await;
        devices
            .values()
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
                true
            })
            .cloned()
            .collect()
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

        // Stage 0: Preparation - Parse CIDR targets
        let mut all_ips: Vec<IpAddr> = Vec::new();
        for target in &targets {
            match parse_cidr(target) {
                Ok(ips) => all_ips.extend(ips),
                Err(e) => {
                    tracing::warn!(target = %target, error = %e, "Failed to parse CIDR");
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
        tracing::info!(job_id = %job_id, total_ips = total_ips, "Stage 0: Prep complete");

        // Stage 1: Host Discovery (parallel with semaphore)
        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency as usize));
        let mut discovery_handles = Vec::new();

        for ip in all_ips.clone() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                let result = discover_host(ip, timeout_ms).await;
                drop(permit);
                result
            });
            discovery_handles.push(handle);
        }

        let mut alive_hosts: Vec<IpAddr> = Vec::new();
        for handle in discovery_handles {
            if let Ok(result) = handle.await {
                if result.alive {
                    alive_hosts.push(result.ip);
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            hosts_alive = alive_hosts.len(),
            "Stage 1: Host discovery complete"
        );

        // Stage 2: MAC/OUI - Skipped for L3 (set mac=null)
        // In L3 environments, we can't get MAC addresses
        tracing::info!(job_id = %job_id, "Stage 2: MAC/OUI skipped (L3 mode)");

        // Stage 3: Port Scan (parallel)
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
                    port_results.insert(ip, open_ports);
                }
            }
        }

        tracing::info!(
            job_id = %job_id,
            hosts_with_ports = port_results.len(),
            "Stage 3: Port scan complete"
        );

        // Stage 4: Discovery Probes (ONVIF, RTSP)
        let mut evidence_map: HashMap<IpAddr, DeviceEvidence> = HashMap::new();
        let mut probe_handles = Vec::new();

        for (ip, open_ports) in &port_results {
            let ip = *ip;
            let open_ports_clone = open_ports.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                // ONVIF probe
                let onvif_found = probe_onvif(ip, timeout_ms).await;

                // RTSP probe on 554 and 8554
                let mut rtsp_found = false;
                for &port in &[554, 8554] {
                    if open_ports_clone.contains(&port) && probe_rtsp(ip, port, timeout_ms).await {
                        rtsp_found = true;
                        break;
                    }
                }

                drop(permit);

                DeviceEvidence {
                    ip,
                    open_ports: open_ports_clone,
                    mac: None,
                    oui_vendor: None,
                    onvif_found,
                    ssdp_found: false, // Not implemented yet
                    mdns_found: false, // Not implemented yet
                }
            });
            probe_handles.push(handle);
        }

        for handle in probe_handles {
            if let Ok(evidence) = handle.await {
                evidence_map.insert(evidence.ip, evidence);
            }
        }

        tracing::info!(
            job_id = %job_id,
            probed_hosts = evidence_map.len(),
            "Stage 4: Discovery probes complete"
        );

        // Stage 5: Scoring
        let mut candidates: Vec<(IpAddr, u32, DeviceEvidence)> = Vec::new();

        for (ip, evidence) in evidence_map {
            let score = calculate_score(&evidence);
            if score >= SCORE_THRESHOLD_VERIFY {
                candidates.push((ip, score, evidence));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by score descending

        tracing::info!(
            job_id = %job_id,
            candidates = candidates.len(),
            "Stage 5: Scoring complete (threshold: {})",
            SCORE_THRESHOLD_VERIFY
        );

        // Stage 6: Verification - Store results (without credential verification)
        // Note: Credential verification requires admin input via separate API
        let now = chrono::Utc::now();
        let mut cameras_found = 0u32;

        {
            let mut devices = self.devices.write().await;

            for (ip, score, evidence) in candidates {
                let device_id = Uuid::new_v4();
                let ip_str = ip.to_string();

                // Determine camera family from evidence
                let family = if evidence.onvif_found {
                    if evidence.open_ports.contains(&2020) {
                        CameraFamily::Tapo // Tapo uses port 2020 for ONVIF
                    } else {
                        CameraFamily::Other
                    }
                } else {
                    CameraFamily::Unknown
                };

                let device = ScannedDevice {
                    device_id,
                    ip: ip_str.clone(),
                    subnet: extract_subnet(&ip_str),
                    mac: None,
                    oui_vendor: None,
                    hostnames: Vec::new(),
                    open_ports: evidence
                        .open_ports
                        .iter()
                        .map(|&p| PortStatus {
                            port: p,
                            status: PortState::Open,
                        })
                        .collect(),
                    score,
                    verified: false, // Not verified until credentials provided
                    manufacturer: None,
                    model: None,
                    firmware: None,
                    family,
                    confidence: calculate_confidence(score),
                    rtsp_uri: None,
                    first_seen: now,
                    last_seen: now,
                };

                devices.insert(ip_str, device);
                cameras_found += 1;
            }
        }

        tracing::info!(
            job_id = %job_id,
            cameras_found = cameras_found,
            "Stage 6: Results stored"
        );

        // Update job summary
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Success;
                job.ended_at = Some(chrono::Utc::now());
                job.summary = Some(JobSummary {
                    total_ips,
                    hosts_alive: alive_hosts.len() as u32,
                    cameras_found,
                    cameras_verified: 0, // Verification requires separate API call
                });
            }
        }

        tracing::info!(
            job_id = %job_id,
            total_ips = total_ips,
            hosts_alive = alive_hosts.len(),
            cameras_found = cameras_found,
            "Scan job completed"
        );

        Ok(())
    }

    /// Verify a device with credentials (Stage 6 completion)
    pub async fn verify_device(
        &self,
        device_ip: &str,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let ip: IpAddr = device_ip
            .parse()
            .map_err(|_| Error::Validation("Invalid IP address".to_string()))?;

        // Try RTSP verification
        let rtsp_uri = scanner::verify_rtsp(ip, 554, username, password, 5000).await;

        if let Some(uri) = rtsp_uri {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.get_mut(device_ip) {
                device.verified = true;
                device.rtsp_uri = Some(uri);
                device.last_seen = chrono::Utc::now();
                device.confidence = 95; // High confidence after verification
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Default for IpcamScan {
    fn default() -> Self {
        Self::new()
    }
}

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
