-- Migration: 025_aranea_registration_fid
-- Description: AraneaRegistrationにFID (施設ID) 列を追加
--
-- FID (Facility ID) はParaclate連携に必要な施設識別子
-- テナント内の施設を特定するために使用

-- ============================================================
-- aranea_registration テーブルにfid列を追加
-- ============================================================
ALTER TABLE aranea_registration
ADD COLUMN fid VARCHAR(16) DEFAULT NULL
COMMENT '施設ID (Facility ID) - Paraclate連携用'
AFTER tid;

-- インデックス追加 (FIDでの検索用)
CREATE INDEX idx_fid ON aranea_registration(fid);
