-- Migration: 020_aranea_registration
-- Description: AraneaRegister用テーブル作成
-- Phase 1: AraneaRegister (Issue #114)
-- DD01_AraneaRegister.md準拠
--
-- ## 目的
-- is22 (Paraclate Server) のaraneaDeviceGate登録情報を永続化
-- CIC (Client Identification Code) を保存し、再起動時の再登録を回避
--
-- ## SSoT原則
-- - config_store: 即時参照用（起動時読み込み、高速アクセス）
-- - aranea_registration: 履歴・監査用（DB永続化）

-- ============================================================
-- aranea_registration テーブル
-- ============================================================
CREATE TABLE IF NOT EXISTS aranea_registration (
    id INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,

    -- LacisID (20桁): [Prefix=3][ProductType=022][MAC=12桁][ProductCode=0000]
    lacis_id VARCHAR(20) NOT NULL UNIQUE,

    -- テナントID (例: T2025120621041161827)
    tid VARCHAR(32) NOT NULL,

    -- Client Identification Code (araneaDeviceGateから取得)
    cic VARCHAR(16) NOT NULL,

    -- デバイスタイプ (ar-is22CamServer)
    device_type VARCHAR(32) NOT NULL DEFAULT 'ar-is22CamServer',

    -- 状態報告先エンドポイント (stateEndpoint)
    state_endpoint VARCHAR(256),

    -- MQTT接続先エンドポイント (双方向デバイス用)
    mqtt_endpoint VARCHAR(256),

    -- 登録日時
    registered_at DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3),

    -- 最終同期日時
    last_sync_at DATETIME(3),

    -- インデックス
    INDEX idx_tid (tid),
    INDEX idx_lacis_id (lacis_id),
    INDEX idx_registered_at (registered_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='is22 AraneaRegister登録情報 (DD01準拠)';

-- ============================================================
-- 初期設定: config_storeにaranea.*キーを登録
-- ============================================================
-- config_storeテーブルが既存の場合のみ実行
-- (config_storeはis22の標準設定テーブル)

-- aranea.lacis_id: 生成済みLacisID
-- aranea.tid: 所属テナントID
-- aranea.cic: 取得済みCIC
-- aranea.registered: 登録完了フラグ
-- aranea.state_endpoint: 状態報告先URL

-- Note: config_storeへの初期値挿入は不要
-- (サービス初期化時に動的に設定されるため)
