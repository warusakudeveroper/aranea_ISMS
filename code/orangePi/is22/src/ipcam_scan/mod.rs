//! IpcamScan - Camera Auto-Discovery
//!
//! Issue #29 [IS22-GAP-004] IpcamScan implementation
//!
//! ## Responsibilities
//!
//! - Multi-stage evidence-based discovery
//! - Stage 0-6: Prep → Host → MAC → Port → Discovery → Score → Verify
//! - No brute-force, proper credentials only

mod types;

pub use types::*;

use crate::error::{Error, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
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
        // Update job status
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(chrono::Utc::now());
            } else {
                return Err(Error::NotFound("Job not found".to_string()));
            }
        }

        // TODO: Implement actual scanning stages
        // Stage 1: Host Discovery
        // Stage 2: MAC/OUI
        // Stage 3: Port Scan
        // Stage 4: Discovery Protocols
        // Stage 5: Scoring
        // Stage 6: Verification

        // For now, just mark as complete
        tokio::time::sleep(Duration::from_secs(1)).await;

        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Success;
                job.ended_at = Some(chrono::Utc::now());
                job.summary = Some(JobSummary {
                    total_ips: 0,
                    hosts_alive: 0,
                    cameras_found: 0,
                    cameras_verified: 0,
                });
            }
        }

        tracing::info!(job_id = %job_id, "Scan job completed");

        Ok(())
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
