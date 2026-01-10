-- Migration: 022_summary_service
-- Description: Summary/GrandSummary用テーブル拡張・新規作成
-- Phase 3: Summary/GrandSummary (Issue #116)
-- DD02_Summary_GrandSummary.md準拠
--
-- ## 概要
-- 検出イベントを定期的に集計し、Summary（間隔ベース）および
-- GrandSummary（時刻指定ベース）を生成・保存する機能

-- ============================================================
-- 1. ai_summary_cache に summary_json カラム追加
-- ============================================================
-- Paraclate送信用のJSONペイロードを保存

ALTER TABLE ai_summary_cache
ADD COLUMN IF NOT EXISTS summary_json JSON NULL
COMMENT 'Paraclate送信用JSONペイロード'
AFTER summary_text;

-- ============================================================
-- 2. scheduled_reports テーブル作成
-- ============================================================
-- Summary/GrandSummaryのスケジュール設定を管理

CREATE TABLE IF NOT EXISTS scheduled_reports (
    schedule_id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- テナント・施設ID
    tid VARCHAR(32) NOT NULL COMMENT 'テナントID',
    fid VARCHAR(32) NOT NULL COMMENT '施設ID',

    -- レポート種別
    report_type ENUM('summary', 'grand_summary') NOT NULL
    COMMENT 'summary=間隔ベース, grand_summary=時刻指定ベース',

    -- Summary用: 生成間隔（分）
    interval_minutes INT NULL
    COMMENT 'Summaryの場合: 60=1時間, 30=30分など',

    -- GrandSummary用: 実行時刻リスト（JST基準）
    scheduled_times JSON NULL
    COMMENT 'GrandSummaryの場合: ["09:00","17:00","21:00"]など (Asia/Tokyo基準)',

    -- 実行履歴
    last_run_at DATETIME(3)
    COMMENT '最終実行日時 (UTC)',
    next_run_at DATETIME(3)
    COMMENT '次回実行予定日時 (UTC)',

    -- 有効/無効
    enabled BOOLEAN DEFAULT TRUE,

    -- タイムスタンプ
    created_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    -- 制約
    UNIQUE KEY uk_tid_fid_type (tid, fid, report_type),
    INDEX idx_next_run (next_run_at, enabled),
    INDEX idx_tid_fid (tid, fid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Summary/GrandSummaryスケジュール設定 (DD02準拠)';

-- ============================================================
-- 3. ai_summary_cache にインデックス追加（検索性能向上）
-- ============================================================

ALTER TABLE ai_summary_cache
ADD INDEX IF NOT EXISTS idx_tid_fid_type (tid, fid, summary_type);

ALTER TABLE ai_summary_cache
ADD INDEX IF NOT EXISTS idx_period (period_start, period_end);
