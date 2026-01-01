-- Migration 006: Add tid (tenant ID) to scan_subnets
-- テナントIDをサブネットに追加（発見されたカメラに継承される）

ALTER TABLE scan_subnets
ADD COLUMN tid VARCHAR(32) NULL AFTER fid;

-- Index for faster tenant lookups
CREATE INDEX idx_subnets_tid ON scan_subnets(tid);

-- Comment: fid + tid はサブネットから発見されたカメラに継承される
-- カメラ登録時にこれらの値がカメラテーブルにコピーされる
