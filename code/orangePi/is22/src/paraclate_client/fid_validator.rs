//! FID Validator - Issue #119: テナント-FID所属検証
//!
//! ## 概要
//! FIDがデバイスの登録TIDに所属しているかを検証する。
//! LacisOath境界違反を防ぐための重要なセキュリティレイヤー。
//!
//! ## 検証ロジック
//! 1. scan_subnetsテーブルで該当FIDを検索
//! 2. そのFIDに紐づくTIDがis22の登録TIDと一致するか確認
//! 3. 不一致の場合はエラーを返す

use crate::config_store::ConfigStore;
use sqlx::MySqlPool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// FID検証エラー
#[derive(Debug, Clone)]
pub enum FidValidationError {
    /// FIDがTIDに所属していない
    NotOwned { fid: String, expected_tid: String, actual_tid: Option<String> },
    /// FIDが見つからない
    NotFound { fid: String },
    /// デバイス未登録
    DeviceNotRegistered,
    /// データベースエラー
    Database(String),
}

impl std::fmt::Display for FidValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotOwned { fid, expected_tid, actual_tid } => {
                write!(
                    f,
                    "FID {} does not belong to TID {}. Actual TID: {}",
                    fid,
                    expected_tid,
                    actual_tid.as_deref().unwrap_or("unknown")
                )
            }
            Self::NotFound { fid } => write!(f, "FID {} not found in scan_subnets", fid),
            Self::DeviceNotRegistered => write!(f, "Device is not registered"),
            Self::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for FidValidationError {}

/// FID所属情報（キャッシュ用）
#[derive(Debug, Clone)]
struct FidOwnership {
    fid: String,
    tid: Option<String>,
}

/// FID Validator
///
/// テナント-FID所属検証を行う。
/// scan_subnetsテーブルを基にFIDとTIDの関係を検証する。
pub struct FidValidator {
    pool: MySqlPool,
    config_store: Arc<ConfigStore>,
    /// キャッシュ（FID -> TID）
    cache: RwLock<HashSet<String>>,
    /// デバイスの登録TID
    device_tid: RwLock<Option<String>>,
}

impl FidValidator {
    /// Create new FidValidator
    pub fn new(pool: MySqlPool, config_store: Arc<ConfigStore>) -> Self {
        Self {
            pool,
            config_store,
            cache: RwLock::new(HashSet::new()),
            device_tid: RwLock::new(None),
        }
    }

    /// デバイスの登録TIDを取得
    async fn get_device_tid(&self) -> Result<String, FidValidationError> {
        // キャッシュチェック
        {
            let cached = self.device_tid.read().await;
            if let Some(ref tid) = *cached {
                return Ok(tid.clone());
            }
        }

        // config_storeから取得
        let tid = self
            .config_store
            .service()
            .get_setting("aranea_tid")
            .await
            .map_err(|e| FidValidationError::Database(e.to_string()))?
            .and_then(|v| v.as_str().map(String::from));

        match tid {
            Some(tid) => {
                // キャッシュに保存
                let mut cached = self.device_tid.write().await;
                *cached = Some(tid.clone());
                Ok(tid)
            }
            None => Err(FidValidationError::DeviceNotRegistered),
        }
    }

    /// FIDがデバイスのTIDに所属しているか検証
    ///
    /// ## エラー
    /// - `FidValidationError::NotOwned`: FIDが別のTIDに所属
    /// - `FidValidationError::NotFound`: FIDが見つからない
    /// - `FidValidationError::DeviceNotRegistered`: デバイス未登録
    pub async fn validate_fid(&self, fid: &str) -> Result<(), FidValidationError> {
        let device_tid = self.get_device_tid().await?;

        // キャッシュチェック
        {
            let cache = self.cache.read().await;
            let cache_key = format!("{}:{}", device_tid, fid);
            if cache.contains(&cache_key) {
                return Ok(());
            }
        }

        // DBから検証
        let result = self.validate_fid_in_db(fid, &device_tid).await?;

        if result {
            // キャッシュに追加
            let mut cache = self.cache.write().await;
            cache.insert(format!("{}:{}", device_tid, fid));
            Ok(())
        } else {
            // FIDの実際のTIDを取得してエラーメッセージに含める
            let actual_tid = self.get_fid_tid(fid).await.ok().flatten();
            Err(FidValidationError::NotOwned {
                fid: fid.to_string(),
                expected_tid: device_tid,
                actual_tid,
            })
        }
    }

    /// 複数FIDを一括検証
    pub async fn validate_fids(&self, fids: &[String]) -> Result<(), FidValidationError> {
        for fid in fids {
            self.validate_fid(fid).await?;
        }
        Ok(())
    }

    /// DBでFIDのTID所属を検証
    async fn validate_fid_in_db(&self, fid: &str, expected_tid: &str) -> Result<bool, FidValidationError> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT tid
            FROM scan_subnets
            WHERE fid = ?
              AND tid = ?
            LIMIT 1
            "#,
        )
        .bind(fid)
        .bind(expected_tid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FidValidationError::Database(e.to_string()))?;

        Ok(result.is_some())
    }

    /// FIDに紐づくTIDを取得
    async fn get_fid_tid(&self, fid: &str) -> Result<Option<String>, FidValidationError> {
        let result: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT tid
            FROM scan_subnets
            WHERE fid = ?
            LIMIT 1
            "#,
        )
        .bind(fid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FidValidationError::Database(e.to_string()))?;

        Ok(result.and_then(|r| r.0))
    }

    /// デバイスに所属する全FIDを取得
    pub async fn get_owned_fids(&self) -> Result<Vec<String>, FidValidationError> {
        let device_tid = self.get_device_tid().await?;

        let results: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT fid
            FROM scan_subnets
            WHERE tid = ?
            ORDER BY fid
            "#,
        )
        .bind(&device_tid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FidValidationError::Database(e.to_string()))?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    /// キャッシュをクリア
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        let mut tid_cache = self.device_tid.write().await;
        *tid_cache = None;
    }

    /// FID検証をスキップするかどうか判定（開発/テスト用）
    ///
    /// 環境変数 `SKIP_FID_VALIDATION=1` で検証をスキップ可能
    pub fn should_skip_validation() -> bool {
        std::env::var("SKIP_FID_VALIDATION")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fid_validation_error_display() {
        let err = FidValidationError::NotOwned {
            fid: "9000".to_string(),
            expected_tid: "T123".to_string(),
            actual_tid: Some("T456".to_string()),
        };
        assert!(err.to_string().contains("9000"));
        assert!(err.to_string().contains("T123"));
        assert!(err.to_string().contains("T456"));

        let err = FidValidationError::NotFound {
            fid: "9999".to_string(),
        };
        assert!(err.to_string().contains("9999"));

        let err = FidValidationError::DeviceNotRegistered;
        assert!(err.to_string().contains("not registered"));
    }

    #[test]
    fn test_should_skip_validation() {
        // Default: false
        std::env::remove_var("SKIP_FID_VALIDATION");
        assert!(!FidValidator::should_skip_validation());

        // Set to 1: true
        std::env::set_var("SKIP_FID_VALIDATION", "1");
        assert!(FidValidator::should_skip_validation());

        // Set to true: true
        std::env::set_var("SKIP_FID_VALIDATION", "true");
        assert!(FidValidator::should_skip_validation());

        // Cleanup
        std::env::remove_var("SKIP_FID_VALIDATION");
    }
}
