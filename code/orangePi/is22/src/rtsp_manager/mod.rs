//! RtspManager - カメラごとのRTSPアクセス制御
//!
//! ## 目的
//!
//! - 同一カメラへの多重RTSP接続を防止
//! - 先行接続が完了するまで短時間待機
//! - タイムアウト時はエラーを返す（過剰リクエスト抑制）

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

/// デフォルト待機タイムアウト（5秒）
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5000;

/// RtspManager - カメラごとのRTSPアクセスを直列化
pub struct RtspManager {
    /// カメラIDごとのロック
    locks: RwLock<HashMap<String, Arc<Mutex<()>>>>,
    /// 待機タイムアウト
    wait_timeout: Duration,
}

impl RtspManager {
    /// 新規作成
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
            wait_timeout: Duration::from_millis(DEFAULT_WAIT_TIMEOUT_MS),
        }
    }

    /// 待機タイムアウトを指定して作成
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
            wait_timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// カメラへのRTSPアクセスを取得（待機あり）
    ///
    /// - 他が使用中なら短時間待機
    /// - タイムアウトしたらエラー
    /// - 返却されたRtspLeaseがDropされると自動解放
    pub async fn acquire(&self, camera_id: &str) -> Result<RtspLease, RtspError> {
        let lock = self.get_or_create_lock(camera_id).await;

        match timeout(self.wait_timeout, lock.clone().lock_owned()).await {
            Ok(guard) => {
                tracing::debug!(camera_id = %camera_id, "RTSP access acquired");
                Ok(RtspLease {
                    camera_id: camera_id.to_string(),
                    _guard: guard,
                })
            }
            Err(_) => {
                tracing::warn!(
                    camera_id = %camera_id,
                    timeout_ms = self.wait_timeout.as_millis(),
                    "RTSP access timeout - camera busy"
                );
                Err(RtspError::Busy)
            }
        }
    }

    /// カメラへのRTSPアクセスを試行（待機なし）
    ///
    /// - 他が使用中なら即None
    pub async fn try_acquire(&self, camera_id: &str) -> Option<RtspLease> {
        let lock = self.get_or_create_lock(camera_id).await;

        match lock.clone().try_lock_owned() {
            Ok(guard) => {
                tracing::debug!(camera_id = %camera_id, "RTSP access acquired (try)");
                Some(RtspLease {
                    camera_id: camera_id.to_string(),
                    _guard: guard,
                })
            }
            Err(_) => {
                tracing::debug!(camera_id = %camera_id, "RTSP access denied - camera busy");
                None
            }
        }
    }

    /// カメラIDに対応するロックを取得（なければ作成）
    async fn get_or_create_lock(&self, camera_id: &str) -> Arc<Mutex<()>> {
        // 読み取りロックでまず確認
        {
            let locks = self.locks.read().await;
            if let Some(lock) = locks.get(camera_id) {
                return lock.clone();
            }
        }

        // なければ書き込みロックで作成
        let mut locks = self.locks.write().await;
        locks
            .entry(camera_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// 登録済みカメラ数を取得（デバッグ用）
    pub async fn lock_count(&self) -> usize {
        self.locks.read().await.len()
    }
}

impl Default for RtspManager {
    fn default() -> Self {
        Self::new()
    }
}

/// RTSPアクセスリース - Dropで自動解放
pub struct RtspLease {
    camera_id: String,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl RtspLease {
    pub fn camera_id(&self) -> &str {
        &self.camera_id
    }
}

impl Drop for RtspLease {
    fn drop(&mut self) {
        tracing::debug!(camera_id = %self.camera_id, "RTSP access released");
    }
}

/// RTSPアクセスエラー
#[derive(Debug, Clone)]
pub enum RtspError {
    /// カメラがビジー（タイムアウト）
    Busy,
}

impl std::fmt::Display for RtspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RtspError::Busy => write!(f, "Camera RTSP busy (timeout)"),
        }
    }
}

impl std::error::Error for RtspError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_release() {
        let manager = RtspManager::new();

        // 取得
        let lease = manager.acquire("cam-001").await.unwrap();
        assert_eq!(lease.camera_id(), "cam-001");

        // Dropで解放
        drop(lease);

        // 再取得可能
        let _lease2 = manager.acquire("cam-001").await.unwrap();
    }

    #[tokio::test]
    async fn test_try_acquire_busy() {
        let manager = RtspManager::new();

        // 1つ目取得
        let _lease1 = manager.acquire("cam-001").await.unwrap();

        // 2つ目はtry_acquireで即失敗
        let result = manager.try_acquire("cam-001").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_different_cameras() {
        let manager = RtspManager::new();

        // 異なるカメラは同時取得可能
        let lease1 = manager.acquire("cam-001").await.unwrap();
        let lease2 = manager.acquire("cam-002").await.unwrap();

        assert_eq!(lease1.camera_id(), "cam-001");
        assert_eq!(lease2.camera_id(), "cam-002");
    }

    #[tokio::test]
    async fn test_timeout() {
        let manager = RtspManager::with_timeout(100); // 100ms

        // 1つ目取得してホールド
        let _lease1 = manager.acquire("cam-001").await.unwrap();

        // 2つ目はタイムアウト
        let result = manager.acquire("cam-001").await;
        assert!(matches!(result, Err(RtspError::Busy)));
    }
}
