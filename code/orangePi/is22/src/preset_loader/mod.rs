//! PresetLoader - AI Analysis Preset Configuration
//!
//! ## Responsibilities
//!
//! - Define 12 standard presets for IS21 analysis
//! - Provide preset lookup by ID
//! - Apply preset configurations to analyze requests
//!
//! ## Design Reference
//!
//! - §2.3 PresetLoader (is22_AI_EVENT_LOG_DESIGN.md)

use crate::ai_client::{AnalyzeRequest, CameraContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Preset ID constants
pub mod preset_ids {
    pub const PERSON_PRIORITY: &str = "person_priority";
    pub const BALANCED: &str = "balanced";
    pub const PARKING: &str = "parking";
    pub const ENTRANCE: &str = "entrance";
    pub const CORRIDOR: &str = "corridor";
    pub const OUTDOOR: &str = "outdoor";
    pub const NIGHT_VISION: &str = "night_vision";
    pub const CROWD: &str = "crowd";
    pub const RETAIL: &str = "retail";
    pub const OFFICE: &str = "office";
    pub const WAREHOUSE: &str = "warehouse";
    pub const CUSTOM: &str = "custom";
}

/// Preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Preset ID
    pub id: String,
    /// Display name (Japanese)
    pub name: String,
    /// Description
    pub description: String,
    /// Version
    pub version: String,

    // Context defaults
    /// Default location type
    pub location_type: Option<String>,
    /// Default distance
    pub distance: Option<String>,
    /// Expected objects to detect
    pub expected_objects: Vec<String>,
    /// Objects to exclude from detection
    pub excluded_objects: Vec<String>,

    // Analysis settings
    /// Enable frame diff analysis
    pub enable_frame_diff: bool,
    /// Return bounding boxes
    pub return_bboxes: bool,
    /// Output schema name
    pub output_schema: Option<String>,

    // v1.9 Threshold overrides (hints_json経由でis21に渡す)
    /// YOLO confidence threshold override (0.2-0.8)
    pub conf_override: Option<f32>,
    /// NMS threshold override (0.3-0.6)
    pub nms_threshold: Option<f32>,
    /// PAR threshold override (0.3-0.8)
    pub par_threshold: Option<f32>,

    // Polling settings (suggested)
    /// Suggested polling interval in seconds
    pub suggested_interval_sec: u32,
}

impl Default for Preset {
    fn default() -> Self {
        Self::balanced()
    }
}

impl Preset {
    /// Create balanced preset (default)
    pub fn balanced() -> Self {
        Self {
            id: preset_ids::BALANCED.to_string(),
            name: "バランス".to_string(),
            description: "汎用的なバランス設定".to_string(),
            version: "1.1.0".to_string(),  // v1.1.0: excluded_objects追加
            location_type: None,
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // v1.1.0: 監視カメラで検知不要なオブジェクトを除外
            // Issue #104: unknown乱発防止（mouse誤検知対策）
            excluded_objects: vec![
                "mouse".to_string(),
                "keyboard".to_string(),
                "cell phone".to_string(),
                "laptop".to_string(),
                "tv".to_string(),
                "book".to_string(),
                "clock".to_string(),
                "vase".to_string(),
                "potted plant".to_string(),
                "teddy bear".to_string(),
                "remote".to_string(),
                "toothbrush".to_string(),
                "hair drier".to_string(),
                "scissors".to_string(),
            ],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 15,
        }
    }

    /// Create person_priority preset
    pub fn person_priority() -> Self {
        Self {
            id: preset_ids::PERSON_PRIORITY.to_string(),
            name: "人物優先".to_string(),
            description: "人物検知を最優先とする設定".to_string(),
            version: "1.0.0".to_string(),
            location_type: None,
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec!["vehicle".to_string(), "animal".to_string()],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("person_detailed".to_string()),
            conf_override: Some(0.3),  // 低めに設定して人物を逃さない
            nms_threshold: None,
            par_threshold: Some(0.4),  // 詳細属性を多く取得
            suggested_interval_sec: 10,
        }
    }

    /// Create parking preset
    pub fn parking() -> Self {
        Self {
            id: preset_ids::PARKING.to_string(),
            name: "駐車場".to_string(),
            description: "駐車場・駐輪場向け（人物+車両）".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("parking".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string(), "vehicle".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("parking".to_string()),
            conf_override: Some(0.45),  // 遠距離で誤検知を抑制
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 20,
        }
    }

    /// Create entrance preset
    pub fn entrance() -> Self {
        Self {
            id: preset_ids::ENTRANCE.to_string(),
            name: "エントランス".to_string(),
            description: "建物入口・エントランス向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("entrance".to_string()),
            distance: Some("near".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec!["vehicle".to_string()],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("person_detailed".to_string()),
            conf_override: Some(0.35),  // 近距離で確実に検出
            nms_threshold: None,
            par_threshold: Some(0.4),  // 詳細属性を取得
            suggested_interval_sec: 10,
        }
    }

    /// Create corridor preset
    pub fn corridor() -> Self {
        Self {
            id: preset_ids::CORRIDOR.to_string(),
            name: "廊下".to_string(),
            description: "廊下・通路向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("corridor".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 15,
        }
    }

    /// Create outdoor preset
    pub fn outdoor() -> Self {
        Self {
            id: preset_ids::OUTDOOR.to_string(),
            name: "屋外".to_string(),
            description: "屋外・外周向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("outdoor".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string(), "vehicle".to_string(), "animal".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: Some(0.5),  // 遠距離・屋外で誤検知抑制
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 20,
        }
    }

    /// Create night_vision preset
    pub fn night_vision() -> Self {
        Self {
            id: preset_ids::NIGHT_VISION.to_string(),
            name: "夜間".to_string(),
            description: "夜間・低照度環境向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: None,
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: Some(0.3),  // 低照度で検出漏れを防ぐ
            nms_threshold: None,
            par_threshold: Some(0.5),  // 低照度では属性認識精度が落ちる
            suggested_interval_sec: 15,
        }
    }

    /// Create crowd preset
    pub fn crowd() -> Self {
        Self {
            id: preset_ids::CROWD.to_string(),
            name: "群衆".to_string(),
            description: "群衆・混雑検知向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("crowd".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: false, // Too many boxes
            output_schema: Some("crowd".to_string()),
            conf_override: Some(0.25),  // 多人数検出のため低閾値
            nms_threshold: Some(0.4),   // 重複排除を弱めに
            par_threshold: None,        // 群衆では属性不要
            suggested_interval_sec: 30,
        }
    }

    /// Create retail preset
    pub fn retail() -> Self {
        Self {
            id: preset_ids::RETAIL.to_string(),
            name: "小売店".to_string(),
            description: "店舗・小売向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("retail".to_string()),
            distance: Some("near".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec!["vehicle".to_string()],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("person_detailed".to_string()),
            conf_override: Some(0.35),  // 近距離で確実に検出
            nms_threshold: None,
            par_threshold: Some(0.4),  // 顧客属性を詳細に取得
            suggested_interval_sec: 10,
        }
    }

    /// Create office preset
    pub fn office() -> Self {
        Self {
            id: preset_ids::OFFICE.to_string(),
            name: "オフィス".to_string(),
            description: "オフィス・会議室向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("office".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            excluded_objects: vec!["vehicle".to_string(), "animal".to_string()],
            enable_frame_diff: false, // Less motion expected
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 30,
        }
    }

    /// Create warehouse preset
    pub fn warehouse() -> Self {
        Self {
            id: preset_ids::WAREHOUSE.to_string(),
            name: "倉庫".to_string(),
            description: "倉庫・物流施設向け".to_string(),
            version: "1.0.0".to_string(),
            location_type: Some("warehouse".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string(), "vehicle".to_string()],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: Some(0.5),  // 遠距離で誤検知抑制
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 20,
        }
    }

    /// Create custom preset (empty template)
    pub fn custom() -> Self {
        Self {
            id: preset_ids::CUSTOM.to_string(),
            name: "カスタム".to_string(),
            description: "カスタム設定（個別設定）".to_string(),
            version: "1.0.0".to_string(),
            location_type: None,
            distance: None,
            expected_objects: vec![],
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 15,
        }
    }

    /// Build CameraContext from preset
    pub fn to_camera_context(&self) -> Option<CameraContext> {
        // Only create context if there are meaningful values
        if self.location_type.is_none()
            && self.distance.is_none()
            && self.expected_objects.is_empty()
            && self.excluded_objects.is_empty()
            && self.conf_override.is_none()
            && self.nms_threshold.is_none()
            && self.par_threshold.is_none()
        {
            return None;
        }

        Some(CameraContext {
            location_type: self.location_type.clone(),
            distance: self.distance.clone(),
            expected_objects: if self.expected_objects.is_empty() {
                None
            } else {
                Some(self.expected_objects.clone())
            },
            excluded_objects: if self.excluded_objects.is_empty() {
                None
            } else {
                Some(self.excluded_objects.clone())
            },
            busy_hours: None,
            quiet_hours: None,
            previous_frame: None,
            conf_override: self.conf_override,
            nms_threshold: self.nms_threshold,
            par_threshold: self.par_threshold,
        })
    }

    /// Apply preset to AnalyzeRequest
    pub fn apply_to_request(&self, request: &mut AnalyzeRequest) {
        request.preset_id = self.id.clone();
        request.preset_version = self.version.clone();
        request.enable_frame_diff = self.enable_frame_diff;
        request.return_bboxes = self.return_bboxes;

        if self.output_schema.is_some() {
            request.output_schema = self.output_schema.clone();
        }

        // Apply camera_context if not already set
        if request.camera_context.is_none() {
            request.camera_context = self.to_camera_context();
        }

        // TODO(autoAttunement): プリセット適用時に統計データを参照して閾値を動的調整
        // 参照: Layout＆AIlog_Settings/AIEventLog_Redesign_v4.md Section 5.6
        // 実装時: カメラの誤検知統計に基づいてconf_overrideを自動調整
        // if let Some(adjusted) = stats_collector.get_adjusted_threshold(&request.camera_id) {
        //     request.hints_json.conf_override = Some(adjusted);
        // }
    }
}

/// PresetLoader service
pub struct PresetLoader {
    /// Preset registry
    presets: HashMap<String, Preset>,
}

impl PresetLoader {
    /// Create new PresetLoader with all standard presets
    pub fn new() -> Self {
        let mut presets = HashMap::new();

        // Register all 12 presets
        let all_presets = vec![
            Preset::person_priority(),
            Preset::balanced(),
            Preset::parking(),
            Preset::entrance(),
            Preset::corridor(),
            Preset::outdoor(),
            Preset::night_vision(),
            Preset::crowd(),
            Preset::retail(),
            Preset::office(),
            Preset::warehouse(),
            Preset::custom(),
        ];

        for preset in all_presets {
            presets.insert(preset.id.clone(), preset);
        }

        Self { presets }
    }

    /// Get preset by ID
    pub fn get(&self, preset_id: &str) -> Option<&Preset> {
        self.presets.get(preset_id)
    }

    /// Get preset by ID or return balanced as default
    pub fn get_or_default(&self, preset_id: &str) -> &Preset {
        self.presets
            .get(preset_id)
            .unwrap_or_else(|| self.presets.get(preset_ids::BALANCED).unwrap())
    }

    /// List all preset IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }

    /// List all presets
    pub fn list_all(&self) -> Vec<&Preset> {
        self.presets.values().collect()
    }

    /// Check if preset exists
    pub fn exists(&self, preset_id: &str) -> bool {
        self.presets.contains_key(preset_id)
    }

    /// Register custom preset
    pub fn register(&mut self, preset: Preset) {
        self.presets.insert(preset.id.clone(), preset);
    }

    /// Create AnalyzeRequest with preset applied
    pub fn create_request(
        &self,
        preset_id: &str,
        camera_id: String,
        captured_at: String,
    ) -> AnalyzeRequest {
        let mut request = AnalyzeRequest {
            camera_id,
            captured_at,
            ..AnalyzeRequest::default()
        };

        if let Some(preset) = self.get(preset_id) {
            preset.apply_to_request(&mut request);
        }

        request
    }
}

impl Default for PresetLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_registered() {
        let loader = PresetLoader::new();
        assert_eq!(loader.presets.len(), 12);

        // Check all preset IDs exist
        assert!(loader.exists(preset_ids::PERSON_PRIORITY));
        assert!(loader.exists(preset_ids::BALANCED));
        assert!(loader.exists(preset_ids::PARKING));
        assert!(loader.exists(preset_ids::ENTRANCE));
        assert!(loader.exists(preset_ids::CORRIDOR));
        assert!(loader.exists(preset_ids::OUTDOOR));
        assert!(loader.exists(preset_ids::NIGHT_VISION));
        assert!(loader.exists(preset_ids::CROWD));
        assert!(loader.exists(preset_ids::RETAIL));
        assert!(loader.exists(preset_ids::OFFICE));
        assert!(loader.exists(preset_ids::WAREHOUSE));
        assert!(loader.exists(preset_ids::CUSTOM));
    }

    #[test]
    fn test_get_or_default() {
        let loader = PresetLoader::new();

        // Known preset
        let preset = loader.get_or_default(preset_ids::PARKING);
        assert_eq!(preset.id, preset_ids::PARKING);

        // Unknown preset should return balanced
        let preset = loader.get_or_default("unknown_preset");
        assert_eq!(preset.id, preset_ids::BALANCED);
    }

    #[test]
    fn test_preset_to_camera_context() {
        let parking = Preset::parking();
        let context = parking.to_camera_context();

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.location_type, Some("parking".to_string()));
        assert_eq!(ctx.distance, Some("far".to_string()));
        assert!(ctx.expected_objects.is_some());
    }

    #[test]
    fn test_create_request() {
        let loader = PresetLoader::new();
        let request = loader.create_request(
            preset_ids::PARKING,
            "cam-001".to_string(),
            "2025-01-01T12:00:00Z".to_string(),
        );

        assert_eq!(request.preset_id, preset_ids::PARKING);
        assert_eq!(request.camera_id, "cam-001");
        assert!(request.enable_frame_diff);
    }
}
