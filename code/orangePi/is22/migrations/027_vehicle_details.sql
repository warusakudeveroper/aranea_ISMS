-- 027_vehicle_details.sql
-- DD19対応: 車両詳細情報カラムの追加
-- IS21からの車両属性認識結果を保存するためのカラム

-- detection_logsテーブルに vehicle_details カラムを追加
-- person_details の後に配置
ALTER TABLE detection_logs
    ADD COLUMN IF NOT EXISTS vehicle_details JSON NULL
        COMMENT '車両詳細情報（車種、色、サイズカテゴリ）' AFTER person_details;

-- コメント: DD19 車両詳細情報対応
--
-- vehicle_details 構造:
-- [
--   {
--     "index": 0,
--     "vehicle_type": "car|truck|bus|motorcycle",
--     "color": { "color": "white", "confidence": 0.85 },
--     "size_category": "small|medium|large"
--   }
-- ]
--
-- person_details と同様に IS21 から送信され、
-- LLMサマリー生成時に車両情報として活用される
