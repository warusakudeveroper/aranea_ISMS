//! PresetLoader - AI Analysis Preset Configuration
//!
//! ## Responsibilities
//!
//! - Define 13 standard presets for IS21 analysis (v2.0)
//! - Provide preset lookup by ID
//! - Apply preset configurations to analyze requests
//!
//! ## Design Reference
//!
//! - §2.3 PresetLoader (is22_AI_EVENT_LOG_DESIGN.md)
//! - PresetRedesign_v2.md (Issue #107)

use crate::ai_client::{AnalyzeRequest, CameraContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// YOLO COCO Labels - 正確なラベル名でIS21と連携
/// Issue #107: "vehicle"/"animal"カテゴリ名ではなくYOLOラベルを使用
pub mod yolo_labels {
    /// 車両系ラベル（vehicleカテゴリ展開）
    pub const VEHICLES: &[&str] = &[
        "bicycle", "car", "motorcycle", "airplane",
        "bus", "train", "truck", "boat"
    ];

    /// 動物系ラベル（animalカテゴリ展開）
    pub const ANIMALS: &[&str] = &[
        "bird", "cat", "dog", "horse", "sheep",
        "cow", "elephant", "bear", "zebra", "giraffe"
    ];

    /// オフィス用品ラベル（室内誤検知対策）
    pub const OFFICE_ITEMS: &[&str] = &[
        "mouse", "keyboard", "laptop", "cell phone",
        "tv", "remote", "book", "clock"
    ];

    /// 室内雑貨ラベル
    pub const INDOOR_MISC: &[&str] = &[
        "vase", "potted plant", "teddy bear",
        "toothbrush", "hair drier", "scissors"
    ];

    /// 家具ラベル
    pub const FURNITURE: &[&str] = &[
        "chair", "couch", "bed", "dining table"
    ];

    /// 飲食器具ラベル
    pub const TABLEWARE: &[&str] = &[
        "bottle", "wine glass", "cup",
        "fork", "knife", "spoon", "bowl"
    ];
}

/// Preset ID constants
/// v2.0: retail, night_vision, warehouse廃止、新規4種追加
pub mod preset_ids {
    pub const PERSON_PRIORITY: &str = "person_priority";
    pub const BALANCED: &str = "balanced";
    pub const PARKING: &str = "parking";
    pub const ENTRANCE: &str = "entrance";
    pub const CORRIDOR: &str = "corridor";
    pub const HOTEL_CORRIDOR: &str = "hotel_corridor";  // v2.0 新規
    pub const OUTDOOR: &str = "outdoor";
    pub const CROWD: &str = "crowd";
    pub const OFFICE: &str = "office";
    pub const RESTAURANT: &str = "restaurant";          // v2.0 新規
    pub const SECURITY_ZONE: &str = "security_zone";    // v2.0 新規
    pub const OBJECT_FOCUS: &str = "object_focus";      // v2.0 新規
    pub const CUSTOM: &str = "custom";
    // 廃止: NIGHT_VISION, RETAIL, WAREHOUSE (v2.0)
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

/// ヘルパー: 複数のラベルスライスを結合してVec<String>に変換
fn combine_labels(slices: &[&[&str]]) -> Vec<String> {
    slices
        .iter()
        .flat_map(|s| s.iter())
        .map(|s| s.to_string())
        .collect()
}

impl Preset {
    /// Create balanced preset (default)
    /// v2.0: 最も安全な汎用プリセット
    pub fn balanced() -> Self {
        Self {
            id: preset_ids::BALANCED.to_string(),
            name: "バランス".to_string(),
            description: "汎用的なバランス設定（推奨デフォルト）".to_string(),
            version: "2.0.0".to_string(),
            location_type: None,
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // v2.0: YOLOラベルを使用（Issue #107）
            excluded_objects: combine_labels(&[
                yolo_labels::OFFICE_ITEMS,
                yolo_labels::INDOOR_MISC,
            ]),
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
    /// v2.0: 人物の詳細属性（PAR: 服装・性別・年齢・持ち物等）を最大限取得
    pub fn person_priority() -> Self {
        Self {
            id: preset_ids::PERSON_PRIORITY.to_string(),
            name: "人物優先".to_string(),
            description: "人物の詳細属性を最大限取得する設定".to_string(),
            version: "2.0.0".to_string(),
            location_type: None,
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // v2.0: YOLOラベルを使用、人物以外を徹底除外
            excluded_objects: combine_labels(&[
                yolo_labels::VEHICLES,
                yolo_labels::ANIMALS,
                yolo_labels::OFFICE_ITEMS,
                yolo_labels::INDOOR_MISC,
                yolo_labels::FURNITURE,
            ]),
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("person_detailed".to_string()),
            conf_override: Some(0.3),
            nms_threshold: None,
            par_threshold: Some(0.35),
            suggested_interval_sec: 10,
        }
    }

    /// Create parking preset
    /// v2.0: 駐車場・倉庫向け（warehouseを吸収）
    pub fn parking() -> Self {
        Self {
            id: preset_ids::PARKING.to_string(),
            name: "駐車場".to_string(),
            description: "駐車場・倉庫・物流施設向け（人物+車両）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("parking".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string(), "vehicle".to_string()],
            // 屋外・広域のため除外なし
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("parking".to_string()),
            conf_override: Some(0.45),
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 20,
        }
    }

    /// Create entrance preset
    /// v2.0: 入口・店舗向け（retailを吸収）
    pub fn entrance() -> Self {
        Self {
            id: preset_ids::ENTRANCE.to_string(),
            name: "エントランス".to_string(),
            description: "建物入口・店舗入口向け（来訪者検知）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("entrance".to_string()),
            distance: Some("near".to_string()),
            expected_objects: vec!["human".to_string()],
            // v2.0: YOLOラベルで車両除外 + 受付周りのオフィス用品除外
            excluded_objects: combine_labels(&[
                yolo_labels::VEHICLES,
                &["mouse", "keyboard", "cell phone"],  // 受付デスク対策
            ]),
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("person_detailed".to_string()),
            conf_override: Some(0.45),  // v2.0.1: 0.35→0.45 unknown flood対策
            nms_threshold: None,
            par_threshold: Some(0.4),
            suggested_interval_sec: 10,
        }
    }

    /// Create corridor preset
    /// v2.0: 一般的な廊下・通路向け
    pub fn corridor() -> Self {
        Self {
            id: preset_ids::CORRIDOR.to_string(),
            name: "廊下".to_string(),
            description: "一般的な廊下・通路向け".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("corridor".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // v2.0: 室内共通の除外リスト
            excluded_objects: combine_labels(&[
                yolo_labels::OFFICE_ITEMS,
            ]),
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 15,
        }
    }

    /// Create hotel_corridor preset
    /// v2.0 新規: ホテル・宿泊施設の廊下向け
    pub fn hotel_corridor() -> Self {
        Self {
            id: preset_ids::HOTEL_CORRIDOR.to_string(),
            name: "ホテル廊下".to_string(),
            description: "宿泊施設廊下向け（人物+ドア状態検知）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("hotel_corridor".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // 客室内什器・オフィス用品を除外
            excluded_objects: combine_labels(&[
                yolo_labels::OFFICE_ITEMS,
                &["chair", "couch"],  // 客室内什器
            ]),
            enable_frame_diff: true,  // ドア開閉検知に必要
            return_bboxes: true,
            output_schema: Some("hotel".to_string()),
            conf_override: Some(0.35),
            nms_threshold: None,
            par_threshold: Some(0.45),
            suggested_interval_sec: 10,
        }
    }

    /// Create outdoor preset
    /// v2.0: 屋外・外周警備向け（動物含む全検出）
    pub fn outdoor() -> Self {
        Self {
            id: preset_ids::OUTDOOR.to_string(),
            name: "屋外".to_string(),
            description: "屋外・外周警備向け（人物+車両+動物）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("outdoor".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string(), "vehicle".to_string(), "animal".to_string()],
            // 屋外のため除外なし
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: None,
            conf_override: Some(0.5),
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 20,
        }
    }

    // night_vision: v2.0で廃止（閾値はスライダー調整可能）

    /// Create crowd preset
    /// v2.0: 群衆・イベント会場向け（正確な人数カウント）
    pub fn crowd() -> Self {
        Self {
            id: preset_ids::CROWD.to_string(),
            name: "群衆".to_string(),
            description: "群衆・イベント会場向け（人数カウント）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("crowd".to_string()),
            distance: Some("far".to_string()),
            expected_objects: vec!["human".to_string()],
            // 人物以外を除外してカウント精度向上
            excluded_objects: combine_labels(&[
                yolo_labels::OFFICE_ITEMS,
            ]),
            enable_frame_diff: true,
            return_bboxes: false,
            output_schema: Some("crowd".to_string()),
            conf_override: Some(0.30),  // v2.0: 0.25→0.30 誤検知抑制
            nms_threshold: Some(0.4),
            par_threshold: None,
            suggested_interval_sec: 30,
        }
    }

    // retail: v2.0で廃止（entranceに統合）

    /// Create office preset
    /// v2.0: オフィス・会議室向け（在席検知）
    pub fn office() -> Self {
        Self {
            id: preset_ids::OFFICE.to_string(),
            name: "オフィス".to_string(),
            description: "オフィス・会議室向け（在席検知）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("office".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // v2.0: YOLOラベルで車両・動物・オフィス用品を除外
            excluded_objects: combine_labels(&[
                yolo_labels::VEHICLES,
                yolo_labels::ANIMALS,
                yolo_labels::OFFICE_ITEMS,
            ]),
            enable_frame_diff: false,
            return_bboxes: true,
            output_schema: None,
            conf_override: None,
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 30,
        }
    }

    // warehouse: v2.0で廃止（parkingに統合）

    /// Create restaurant preset
    /// v2.0 新規: 飲食店ホール向け（正確な人数カウント）
    pub fn restaurant() -> Self {
        Self {
            id: preset_ids::RESTAURANT.to_string(),
            name: "飲食店ホール".to_string(),
            description: "飲食店向け（正確な人数カウント）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("restaurant".to_string()),
            distance: Some("medium".to_string()),
            expected_objects: vec!["human".to_string()],
            // 飲食店固有: オフィス用品 + 什器 + 飲食器具を除外
            excluded_objects: combine_labels(&[
                yolo_labels::OFFICE_ITEMS,
                yolo_labels::FURNITURE,
                yolo_labels::TABLEWARE,
            ]),
            enable_frame_diff: true,
            return_bboxes: false,  // カウントのみ、bbox不要
            output_schema: Some("count".to_string()),
            conf_override: Some(0.35),
            nms_threshold: Some(0.5),  // 重複排除を厳密に
            par_threshold: None,
            suggested_interval_sec: 15,
        }
    }

    /// Create security_zone preset
    /// v2.0 新規: 重要警戒区域向け（あらゆる変化を厳密検知）
    pub fn security_zone() -> Self {
        Self {
            id: preset_ids::SECURITY_ZONE.to_string(),
            name: "警戒区域".to_string(),
            description: "重要エリア向け（あらゆる変化を厳密検知）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("security".to_string()),
            distance: Some("medium".to_string()),
            // 全オブジェクトを検出対象
            expected_objects: vec![
                "human".to_string(),
                "vehicle".to_string(),
                "animal".to_string(),
            ],
            // 除外なし - 全て検知
            excluded_objects: vec![],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("security".to_string()),
            conf_override: Some(0.25),  // 低閾値で見逃し防止
            nms_threshold: Some(0.3),
            par_threshold: Some(0.3),
            suggested_interval_sec: 5,  // 高頻度ポーリング
        }
    }

    /// Create object_focus preset
    /// v2.0 新規: 物品監視向け（人を無視、静的物体変化検知）
    pub fn object_focus() -> Self {
        Self {
            id: preset_ids::OBJECT_FOCUS.to_string(),
            name: "対物検知".to_string(),
            description: "物品監視向け（人を無視、背景変化検知）".to_string(),
            version: "2.0.0".to_string(),
            location_type: Some("object".to_string()),
            distance: Some("near".to_string()),
            // 空 = フィルタなし（後でperson除外）
            expected_objects: vec![],
            // 人を除外（逆転の発想）
            excluded_objects: vec!["person".to_string()],
            enable_frame_diff: true,
            return_bboxes: true,
            output_schema: Some("object".to_string()),
            conf_override: Some(0.4),
            nms_threshold: None,
            par_threshold: None,
            suggested_interval_sec: 30,
        }
    }

    // === 以下廃止プリセットのスタブ（後方互換用） ===
    // 既存DBに残っている場合にbalancedにフォールバック

    #[doc(hidden)]
    #[deprecated(since = "2.0.0", note = "Use entrance instead")]
    pub fn retail() -> Self {
        Self::entrance()  // entranceに統合
    }

    #[doc(hidden)]
    #[deprecated(since = "2.0.0", note = "Use balanced with low conf_override")]
    pub fn night_vision() -> Self {
        Self::balanced()  // balancedで代替
    }

    #[doc(hidden)]
    #[deprecated(since = "2.0.0", note = "Use parking instead")]
    pub fn warehouse() -> Self {
        Self::parking()  // parkingに統合
    }

    /// Create custom preset (empty template)
    /// v2.0: ユーザー定義用テンプレート
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
    /// v2.0: 13プリセット（廃止3種除外、新規4種追加）
    pub fn new() -> Self {
        let mut presets = HashMap::new();

        // Register all 13 active presets (v2.0)
        let all_presets = vec![
            Preset::person_priority(),
            Preset::balanced(),
            Preset::parking(),
            Preset::entrance(),
            Preset::corridor(),
            Preset::hotel_corridor(),    // v2.0 新規
            Preset::outdoor(),
            Preset::crowd(),
            Preset::office(),
            Preset::restaurant(),        // v2.0 新規
            Preset::security_zone(),     // v2.0 新規
            Preset::object_focus(),      // v2.0 新規
            Preset::custom(),
        ];
        // 廃止: night_vision, retail, warehouse (v2.0)
        // これらのスタブ関数は存在するが登録しない（フォールバック用）

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
        // v2.0: 13プリセット（廃止3種除外、新規4種追加）
        assert_eq!(loader.presets.len(), 13);

        // Check all active preset IDs exist
        assert!(loader.exists(preset_ids::PERSON_PRIORITY));
        assert!(loader.exists(preset_ids::BALANCED));
        assert!(loader.exists(preset_ids::PARKING));
        assert!(loader.exists(preset_ids::ENTRANCE));
        assert!(loader.exists(preset_ids::CORRIDOR));
        assert!(loader.exists(preset_ids::HOTEL_CORRIDOR));  // v2.0 新規
        assert!(loader.exists(preset_ids::OUTDOOR));
        assert!(loader.exists(preset_ids::CROWD));
        assert!(loader.exists(preset_ids::OFFICE));
        assert!(loader.exists(preset_ids::RESTAURANT));      // v2.0 新規
        assert!(loader.exists(preset_ids::SECURITY_ZONE));   // v2.0 新規
        assert!(loader.exists(preset_ids::OBJECT_FOCUS));    // v2.0 新規
        assert!(loader.exists(preset_ids::CUSTOM));

        // v2.0: 廃止プリセットは登録されていないことを確認
        assert!(!loader.exists("night_vision"));
        assert!(!loader.exists("retail"));
        assert!(!loader.exists("warehouse"));
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

    // v2.0: 新規プリセットのテスト
    #[test]
    fn test_hotel_corridor_preset() {
        let preset = Preset::hotel_corridor();
        assert_eq!(preset.id, preset_ids::HOTEL_CORRIDOR);
        assert_eq!(preset.version, "2.0.0");
        assert!(preset.enable_frame_diff);  // ドア開閉検知に必要
        // オフィス用品が除外されていることを確認
        assert!(preset.excluded_objects.contains(&"mouse".to_string()));
        assert!(preset.excluded_objects.contains(&"keyboard".to_string()));
    }

    #[test]
    fn test_restaurant_preset() {
        let preset = Preset::restaurant();
        assert_eq!(preset.id, preset_ids::RESTAURANT);
        assert!(!preset.return_bboxes);  // カウントのみ
        // 飲食器具が除外されていることを確認
        assert!(preset.excluded_objects.contains(&"fork".to_string()));
        assert!(preset.excluded_objects.contains(&"cup".to_string()));
    }

    #[test]
    fn test_security_zone_preset() {
        let preset = Preset::security_zone();
        assert_eq!(preset.id, preset_ids::SECURITY_ZONE);
        // 除外なし（全検知）
        assert!(preset.excluded_objects.is_empty());
        // 低閾値で見逃し防止
        assert_eq!(preset.conf_override, Some(0.25));
        // 高頻度ポーリング
        assert_eq!(preset.suggested_interval_sec, 5);
    }

    #[test]
    fn test_object_focus_preset() {
        let preset = Preset::object_focus();
        assert_eq!(preset.id, preset_ids::OBJECT_FOCUS);
        // 人を除外（逆転の発想）
        assert!(preset.excluded_objects.contains(&"person".to_string()));
        assert_eq!(preset.excluded_objects.len(), 1);
    }

    // v2.0: YOLOラベル使用の検証
    #[test]
    fn test_yolo_labels_in_excluded_objects() {
        let balanced = Preset::balanced();
        // "vehicle"カテゴリではなくYOLOラベル"car"等を使用していないことを確認
        // （balancedはオフィス用品のみ除外）
        assert!(!balanced.excluded_objects.contains(&"vehicle".to_string()));
        assert!(!balanced.excluded_objects.contains(&"animal".to_string()));
        // YOLOラベル"mouse"（コンピューターマウス）が含まれることを確認
        assert!(balanced.excluded_objects.contains(&"mouse".to_string()));

        let office = Preset::office();
        // officeプリセットはYOLOラベルで車両を除外
        assert!(office.excluded_objects.contains(&"car".to_string()));
        assert!(office.excluded_objects.contains(&"truck".to_string()));
        // "vehicle"カテゴリ名は使用しない
        assert!(!office.excluded_objects.contains(&"vehicle".to_string()));
    }

    // v2.0: 廃止プリセットスタブのフォールバック検証
    #[test]
    #[allow(deprecated)]
    fn test_deprecated_preset_stubs() {
        // retail -> entrance
        let retail = Preset::retail();
        assert_eq!(retail.id, preset_ids::ENTRANCE);

        // night_vision -> balanced
        let night_vision = Preset::night_vision();
        assert_eq!(night_vision.id, preset_ids::BALANCED);

        // warehouse -> parking
        let warehouse = Preset::warehouse();
        assert_eq!(warehouse.id, preset_ids::PARKING);
    }
}
