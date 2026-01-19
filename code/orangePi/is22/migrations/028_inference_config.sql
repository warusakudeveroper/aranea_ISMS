-- 028_inference_config.sql
-- camera_context(LLM用テキスト)とinference_config(IS21用JSON)の分離
--
-- 設計背景:
-- - camera_context: LLM/Paraclateサマリー生成用の説明テキスト
-- - inference_config: IS21推論パラメータ（プリセットとマージして使用）
--
-- inference_config構造:
-- {
--   "conf_override": 0.28,      // YOLO信頼度閾値
--   "nms_threshold": 0.45,      // NMS閾値
--   "par_threshold": 0.4,       // PAR閾値
--   "expected_objects": ["human", "vehicle"],
--   "excluded_objects": ["bird", "cat"],
--   "distance": "far",
--   "location_type": "parking"
-- }

-- camerasテーブルにinference_configカラムを追加
ALTER TABLE cameras
    ADD COLUMN IF NOT EXISTS inference_config JSON NULL
        COMMENT 'IS21推論パラメータ（プリセットとマージ）' AFTER camera_context;

-- camera_contextカラムのコメントを更新（TEXT型のまま）
ALTER TABLE cameras
    MODIFY COLUMN camera_context TEXT NULL
        COMMENT 'LLM/Paraclateサマリー生成用の説明テキスト';
