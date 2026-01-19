# Access Absorber データベーススキーマ

## 1. テーブル定義

### 1.1 camera_access_limits（ブランド別デフォルト設定）

```sql
CREATE TABLE camera_access_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    family VARCHAR(32) NOT NULL UNIQUE,
    display_name VARCHAR(64) NOT NULL,
    max_concurrent_streams TINYINT UNSIGNED NOT NULL DEFAULT 3,
    min_reconnect_interval_ms INT UNSIGNED NOT NULL DEFAULT 0,
    require_exclusive_lock BOOLEAN NOT NULL DEFAULT FALSE,
    connection_timeout_ms INT UNSIGNED NOT NULL DEFAULT 5000,
    notes TEXT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    INDEX idx_family (family)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 初期データ
INSERT INTO camera_access_limits
(family, display_name, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, notes)
VALUES
('tapo',      'TP-link Tapo',     1, 0,    TRUE,  '並列接続不可。go2rtcプロキシ推奨'),
('vigi',      'TP-link VIGI',     3, 0,    FALSE, '3並列まで安定。5並列以上で不安定'),
('nvt',       'NVT (JOOAN等)',    2, 2000, FALSE, '2秒間隔必須。急速再接続5%成功率'),
('hikvision', 'Hikvision',        4, 0,    FALSE, '推定値'),
('dahua',     'Dahua',            4, 0,    FALSE, '推定値'),
('axis',      'Axis',             8, 0,    FALSE, '推定値'),
('unknown',   '不明',             1, 1000, TRUE,  '安全側デフォルト');
```

### 1.2 camera_stream_sessions（アクティブセッション管理）

```sql
CREATE TABLE camera_stream_sessions (
    session_id VARCHAR(64) PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    stream_type ENUM('main', 'sub') NOT NULL DEFAULT 'main',
    purpose ENUM('click_modal', 'suggest_play', 'polling', 'snapshot', 'health_check') NOT NULL,
    client_id VARCHAR(128) NOT NULL,
    started_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3),
    last_heartbeat_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    status ENUM('active', 'releasing', 'expired') NOT NULL DEFAULT 'active',

    INDEX idx_camera_id (camera_id),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at),

    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 1.3 camera_connection_events（接続イベントログ）

```sql
CREATE TABLE camera_connection_events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    event_type ENUM(
        'connect_success',
        'connect_blocked_concurrent',
        'connect_blocked_interval',
        'connect_timeout',
        'disconnect_normal',
        'disconnect_preempted',
        'disconnect_timeout'
    ) NOT NULL,
    purpose ENUM('click_modal', 'suggest_play', 'polling', 'snapshot', 'health_check'),
    client_id VARCHAR(128),
    blocked_reason TEXT,
    wait_time_ms INT UNSIGNED,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),

    INDEX idx_camera_id_created (camera_id, created_at),
    INDEX idx_event_type (event_type),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 1.4 facility_camera_limits（施設別オーバーライド）

```sql
CREATE TABLE facility_camera_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    fid VARCHAR(4) NOT NULL,
    family VARCHAR(32) NOT NULL,
    max_concurrent_streams TINYINT UNSIGNED,
    min_reconnect_interval_ms INT UNSIGNED,
    require_exclusive_lock BOOLEAN,
    notes TEXT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),

    UNIQUE KEY uk_fid_family (fid, family),
    INDEX idx_fid (fid)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

## 2. camerasテーブル拡張

```sql
ALTER TABLE cameras ADD COLUMN access_limit_override JSON DEFAULT NULL;
-- 例: {"max_concurrent_streams": 2, "min_reconnect_interval_ms": 1000}
```

---

## 3. ビュー定義

### 3.1 v_camera_effective_limits（実効制限値）

```sql
CREATE VIEW v_camera_effective_limits AS
SELECT
    c.camera_id,
    c.name,
    c.family,
    c.fid,

    -- max_concurrent_streams (優先順: カメラ個別 > 施設別 > ブランド別)
    COALESCE(
        JSON_UNQUOTE(JSON_EXTRACT(c.access_limit_override, '$.max_concurrent_streams')),
        fcl.max_concurrent_streams,
        cal.max_concurrent_streams,
        1
    ) AS max_concurrent_streams,

    -- min_reconnect_interval_ms
    COALESCE(
        JSON_UNQUOTE(JSON_EXTRACT(c.access_limit_override, '$.min_reconnect_interval_ms')),
        fcl.min_reconnect_interval_ms,
        cal.min_reconnect_interval_ms,
        1000
    ) AS min_reconnect_interval_ms,

    -- require_exclusive_lock
    COALESCE(
        JSON_UNQUOTE(JSON_EXTRACT(c.access_limit_override, '$.require_exclusive_lock')),
        fcl.require_exclusive_lock,
        cal.require_exclusive_lock,
        TRUE
    ) AS require_exclusive_lock,

    -- 設定ソース
    CASE
        WHEN c.access_limit_override IS NOT NULL THEN 'camera_override'
        WHEN fcl.id IS NOT NULL THEN 'facility_override'
        WHEN cal.id IS NOT NULL THEN 'brand_default'
        ELSE 'system_default'
    END AS config_source

FROM cameras c
LEFT JOIN camera_access_limits cal ON c.family = cal.family
LEFT JOIN facility_camera_limits fcl ON c.fid = fcl.fid AND c.family = fcl.family
WHERE c.deleted_at IS NULL;
```

### 3.2 v_camera_stream_status（ストリーム状態）

```sql
CREATE VIEW v_camera_stream_status AS
SELECT
    c.camera_id,
    c.name,
    c.family,
    cel.max_concurrent_streams,
    cel.min_reconnect_interval_ms,
    cel.require_exclusive_lock,

    -- アクティブストリーム数
    (
        SELECT COUNT(*)
        FROM camera_stream_sessions css
        WHERE css.camera_id = c.camera_id
        AND css.status = 'active'
    ) AS active_stream_count,

    -- 利用可能スロット数
    cel.max_concurrent_streams - (
        SELECT COUNT(*)
        FROM camera_stream_sessions css
        WHERE css.camera_id = c.camera_id
        AND css.status = 'active'
    ) AS available_slots,

    -- 接続可能か
    CASE
        WHEN (
            SELECT COUNT(*)
            FROM camera_stream_sessions css
            WHERE css.camera_id = c.camera_id
            AND css.status = 'active'
        ) < cel.max_concurrent_streams THEN TRUE
        ELSE FALSE
    END AS can_connect

FROM cameras c
JOIN v_camera_effective_limits cel ON c.camera_id = cel.camera_id
WHERE c.deleted_at IS NULL;
```

---

## 4. ストアドプロシージャ

### 4.1 sp_acquire_stream_session

```sql
DELIMITER //

CREATE PROCEDURE sp_acquire_stream_session(
    IN p_camera_id VARCHAR(64),
    IN p_stream_type VARCHAR(8),
    IN p_purpose VARCHAR(32),
    IN p_client_id VARCHAR(128),
    OUT p_session_id VARCHAR(64),
    OUT p_result_code INT,
    OUT p_result_message TEXT
)
BEGIN
    DECLARE v_max_concurrent INT;
    DECLARE v_active_count INT;
    DECLARE v_require_lock BOOLEAN;

    -- トランザクション開始
    START TRANSACTION;

    -- 実効制限値取得
    SELECT max_concurrent_streams, require_exclusive_lock
    INTO v_max_concurrent, v_require_lock
    FROM v_camera_effective_limits
    WHERE camera_id = p_camera_id
    FOR UPDATE;

    -- アクティブセッション数取得
    SELECT COUNT(*)
    INTO v_active_count
    FROM camera_stream_sessions
    WHERE camera_id = p_camera_id
    AND status = 'active';

    -- 制限チェック
    IF v_active_count >= v_max_concurrent THEN
        SET p_result_code = 429;
        SET p_result_message = CONCAT('Concurrent limit reached: ', v_active_count, '/', v_max_concurrent);
        SET p_session_id = NULL;

        -- イベントログ
        INSERT INTO camera_connection_events
        (camera_id, event_type, purpose, client_id, blocked_reason)
        VALUES
        (p_camera_id, 'connect_blocked_concurrent', p_purpose, p_client_id, p_result_message);

        ROLLBACK;
    ELSE
        -- セッション作成
        SET p_session_id = CONCAT('sess-', UUID());

        INSERT INTO camera_stream_sessions
        (session_id, camera_id, stream_type, purpose, client_id, expires_at)
        VALUES
        (p_session_id, p_camera_id, p_stream_type, p_purpose, p_client_id, DATE_ADD(NOW(), INTERVAL 1 HOUR));

        -- イベントログ
        INSERT INTO camera_connection_events
        (camera_id, event_type, purpose, client_id)
        VALUES
        (p_camera_id, 'connect_success', p_purpose, p_client_id);

        SET p_result_code = 200;
        SET p_result_message = 'Success';

        COMMIT;
    END IF;
END //

DELIMITER ;
```

### 4.2 sp_release_stream_session

```sql
DELIMITER //

CREATE PROCEDURE sp_release_stream_session(
    IN p_session_id VARCHAR(64),
    OUT p_result_code INT
)
BEGIN
    DECLARE v_camera_id VARCHAR(64);
    DECLARE v_purpose VARCHAR(32);
    DECLARE v_client_id VARCHAR(128);

    -- セッション情報取得
    SELECT camera_id, purpose, client_id
    INTO v_camera_id, v_purpose, v_client_id
    FROM camera_stream_sessions
    WHERE session_id = p_session_id;

    IF v_camera_id IS NULL THEN
        SET p_result_code = 404;
    ELSE
        -- セッション削除
        DELETE FROM camera_stream_sessions WHERE session_id = p_session_id;

        -- イベントログ
        INSERT INTO camera_connection_events
        (camera_id, event_type, purpose, client_id)
        VALUES
        (v_camera_id, 'disconnect_normal', v_purpose, v_client_id);

        SET p_result_code = 200;
    END IF;
END //

DELIMITER ;
```

---

## 5. 定期クリーンアップジョブ

```sql
-- 期限切れセッションのクリーンアップ（1分ごと実行推奨）
CREATE EVENT IF NOT EXISTS evt_cleanup_expired_sessions
ON SCHEDULE EVERY 1 MINUTE
DO
BEGIN
    -- 期限切れセッションをログに記録
    INSERT INTO camera_connection_events
    (camera_id, event_type, purpose, client_id)
    SELECT camera_id, 'disconnect_timeout', purpose, client_id
    FROM camera_stream_sessions
    WHERE status = 'active'
    AND (expires_at < NOW() OR last_heartbeat_at < DATE_SUB(NOW(), INTERVAL 2 MINUTE));

    -- 期限切れセッションを削除
    DELETE FROM camera_stream_sessions
    WHERE status = 'active'
    AND (expires_at < NOW() OR last_heartbeat_at < DATE_SUB(NOW(), INTERVAL 2 MINUTE));
END;
```

---

## 6. マイグレーションSQL

```sql
-- Migration: 030_access_absorber.sql

-- 1. ブランド別制限テーブル
CREATE TABLE IF NOT EXISTS camera_access_limits (
    id INT AUTO_INCREMENT PRIMARY KEY,
    family VARCHAR(32) NOT NULL UNIQUE,
    display_name VARCHAR(64) NOT NULL,
    max_concurrent_streams TINYINT UNSIGNED NOT NULL DEFAULT 3,
    min_reconnect_interval_ms INT UNSIGNED NOT NULL DEFAULT 0,
    require_exclusive_lock BOOLEAN NOT NULL DEFAULT FALSE,
    connection_timeout_ms INT UNSIGNED NOT NULL DEFAULT 5000,
    notes TEXT,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 2. 初期データ投入
INSERT IGNORE INTO camera_access_limits
(family, display_name, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, notes)
VALUES
('tapo',      'TP-link Tapo',     1, 0,    TRUE,  '並列接続不可'),
('vigi',      'TP-link VIGI',     3, 0,    FALSE, '3並列まで安定'),
('nvt',       'NVT (JOOAN等)',    2, 2000, FALSE, '2秒間隔必須'),
('hikvision', 'Hikvision',        4, 0,    FALSE, '推定値'),
('dahua',     'Dahua',            4, 0,    FALSE, '推定値'),
('unknown',   '不明',             1, 1000, TRUE,  '安全側デフォルト');

-- 3. ストリームセッションテーブル
CREATE TABLE IF NOT EXISTS camera_stream_sessions (
    session_id VARCHAR(64) PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    stream_type ENUM('main', 'sub') NOT NULL DEFAULT 'main',
    purpose ENUM('click_modal', 'suggest_play', 'polling', 'snapshot', 'health_check') NOT NULL,
    client_id VARCHAR(128) NOT NULL,
    started_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    expires_at DATETIME(3),
    last_heartbeat_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    status ENUM('active', 'releasing', 'expired') NOT NULL DEFAULT 'active',
    INDEX idx_camera_id (camera_id),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 4. 接続イベントログテーブル
CREATE TABLE IF NOT EXISTS camera_connection_events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    camera_id VARCHAR(64) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    purpose VARCHAR(32),
    client_id VARCHAR(128),
    blocked_reason TEXT,
    wait_time_ms INT UNSIGNED,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    INDEX idx_camera_id_created (camera_id, created_at),
    INDEX idx_event_type (event_type),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 5. camerasテーブル拡張
ALTER TABLE cameras ADD COLUMN IF NOT EXISTS access_limit_override JSON DEFAULT NULL;
```
