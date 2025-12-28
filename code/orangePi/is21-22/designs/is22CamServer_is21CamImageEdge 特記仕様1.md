
# (1) MariaDB DDL（CREATE TABLE一式）

> 前提：MariaDB（InnoDB / utf8mb4）
> 目的：
>
> * `frames` に「毎回の取得結果（何もなし含む）」を保存
> * `events` に「連続検知を束ねたイベント」を保存（検索の主役）
> * タグは `*_tags` で正規化して高速検索に対応（JSON検索に依存しない）
> * Driveは file_id を保持（リンクは is22 側で組み立てる）

```sql
-- =========================================================
-- Database
-- =========================================================
CREATE DATABASE IF NOT EXISTS camserver
  DEFAULT CHARACTER SET utf8mb4
  DEFAULT COLLATE utf8mb4_unicode_ci;

USE camserver;

-- =========================================================
-- Cameras
-- =========================================================
CREATE TABLE IF NOT EXISTS cameras (
  camera_id            VARCHAR(64)  NOT NULL,
  display_name         VARCHAR(128) NOT NULL,
  location_label       VARCHAR(128) NULL,         -- 例: "2F corridor"
  site_label           VARCHAR(64)  NULL,         -- 例: "tokyo_hq", "kyoto"
  network_zone         ENUM('local','vpn') NOT NULL DEFAULT 'local',
  enabled              TINYINT(1) NOT NULL DEFAULT 1,

  polling_interval_sec SMALLINT UNSIGNED NOT NULL DEFAULT 60, -- 目標 60秒
  preferred_stream     ENUM('main','sub') NOT NULL DEFAULT 'main',

  notes                VARCHAR(255) NULL,

  created_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (camera_id),
  KEY idx_cameras_enabled (enabled),
  KEY idx_cameras_zone (network_zone)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ストリーム情報（複数URLを持てる）
CREATE TABLE IF NOT EXISTS camera_streams (
  stream_id        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  camera_id        VARCHAR(64) NOT NULL,
  stream_kind      ENUM('rtsp_main','rtsp_sub','snapshot_http','onvif') NOT NULL,
  url              VARCHAR(1024) NOT NULL, -- RTSP/HTTP/ONVIF endpoint（資格情報は埋め込まない想定）
  transport        ENUM('tcp','udp','auto') NOT NULL DEFAULT 'auto',
  is_primary       TINYINT(1) NOT NULL DEFAULT 0,

  created_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (stream_id),
  UNIQUE KEY uq_camera_stream_kind (camera_id, stream_kind),
  KEY idx_camera_stream_primary (camera_id, is_primary),

  CONSTRAINT fk_camera_streams_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 資格情報（必要なら暗号化して保存）
CREATE TABLE IF NOT EXISTS camera_auth (
  camera_id        VARCHAR(64) NOT NULL,
  username         VARCHAR(128) NULL,
  password_enc     VARBINARY(512) NULL,   -- アプリ側で暗号化したものを格納（生パスワードは非推奨）
  auth_note        VARCHAR(255) NULL,

  updated_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (camera_id),
  CONSTRAINT fk_camera_auth_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- カメラ個別オーバーライド（解像度混在対策・マスク等）
CREATE TABLE IF NOT EXISTS camera_overrides (
  camera_id          VARCHAR(64) NOT NULL,

  infer_width        SMALLINT UNSIGNED NULL, -- 例: 640 or 960（nullならグローバル設定）
  diff_width         SMALLINT UNSIGNED NULL, -- 例: 320 or 160
  force_infer_every_n SMALLINT UNSIGNED NULL, -- 例: 60（差分が小さくてもN回に1回推論）

  ignore_mask_json   JSON NULL,  -- 例: 差分除外領域ポリゴン（後で）
  shelf_roi_json     JSON NULL,  -- 例: 在庫監視の棚ROI（後で）

  updated_at         TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (camera_id),
  CONSTRAINT fk_camera_overrides_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE,

  CHECK (ignore_mask_json IS NULL OR JSON_VALID(ignore_mask_json)),
  CHECK (shelf_roi_json IS NULL OR JSON_VALID(shelf_roi_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Schema versions (is22が管理してis21へ配布するschema)
-- =========================================================
CREATE TABLE IF NOT EXISTS schema_versions (
  schema_version   VARCHAR(64) NOT NULL,
  is_active        TINYINT(1) NOT NULL DEFAULT 0,
  schema_json      JSON NOT NULL,

  created_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (schema_version),
  KEY idx_schema_active (is_active),

  CHECK (JSON_VALID(schema_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Frames (毎回のスナップ取得結果：何もなしも含む)
-- =========================================================
CREATE TABLE IF NOT EXISTS frames (
  frame_id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  frame_uuid        CHAR(36) NOT NULL,                -- 外部参照用
  camera_id         VARCHAR(64) NOT NULL,

  captured_at       DATETIME(3) NOT NULL,             -- UTC推奨（アプリで統一）
  collector_status  ENUM('ok','timeout','error') NOT NULL DEFAULT 'ok',
  error_code        VARCHAR(64) NULL,
  error_message     VARCHAR(255) NULL,

  -- 差分判定（is22側）
  diff_ratio        DECIMAL(9,6) NULL,                -- 例: 0.000123
  luma_delta        SMALLINT NULL,                    -- 平均輝度差（-255..255想定）
  blur_score        DECIMAL(10,3) NULL,               -- 任意

  -- 推論結果（is21側）
  analyzed          TINYINT(1) NOT NULL DEFAULT 0,     -- 推論実施したか
  detected          TINYINT(1) NOT NULL DEFAULT 0,     -- 何か検知（no_eventは0）
  primary_event     VARCHAR(32) NOT NULL DEFAULT 'none', -- human/animal/vehicle/hazard.../unknown/none
  unknown_flag      TINYINT(1) NOT NULL DEFAULT 0,
  severity          TINYINT UNSIGNED NOT NULL DEFAULT 0, -- 0-3
  confidence        DECIMAL(4,3) NULL,                -- 0.000-1.000
  count_hint        SMALLINT UNSIGNED NULL,            -- 人数/動物数のざっくり

  -- 画像情報（fullはDrive、inferは送信・一時用）
  full_w            SMALLINT UNSIGNED NULL,
  full_h            SMALLINT UNSIGNED NULL,
  infer_w           SMALLINT UNSIGNED NULL,
  infer_h           SMALLINT UNSIGNED NULL,
  letterbox         TINYINT(1) NOT NULL DEFAULT 0,
  letterbox_top     SMALLINT UNSIGNED NULL,
  letterbox_left    SMALLINT UNSIGNED NULL,
  letterbox_bottom  SMALLINT UNSIGNED NULL,
  letterbox_right   SMALLINT UNSIGNED NULL,

  infer_hash64_hex  CHAR(16) NULL,                    -- xxhash64等（任意、重複排除用）

  -- 生JSON（検索キーは別列＋tagsテーブルで持つ）
  result_json       JSON NULL,
  tags_json         JSON NULL,

  -- Drive（保存した場合のみ）
  drive_file_id     VARCHAR(128) NULL,
  retention_class   ENUM('normal','quarantine','case') NOT NULL DEFAULT 'normal',

  created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (frame_id),
  UNIQUE KEY uq_frames_uuid (frame_uuid),
  KEY idx_frames_camera_time (camera_id, captured_at),
  KEY idx_frames_time (captured_at),
  KEY idx_frames_detected_time (detected, captured_at),
  KEY idx_frames_event_time (primary_event, captured_at),
  KEY idx_frames_severity_time (severity, captured_at),
  KEY idx_frames_unknown_time (unknown_flag, captured_at),

  CONSTRAINT fk_frames_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE,

  CHECK (result_json IS NULL OR JSON_VALID(result_json)),
  CHECK (tags_json IS NULL OR JSON_VALID(tags_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- フレームに付いたタグ（検索高速化）
CREATE TABLE IF NOT EXISTS frame_tags (
  frame_id       BIGINT UNSIGNED NOT NULL,
  tag_id         VARCHAR(128) NOT NULL,     -- schema.jsonのtag.id
  tag_group      VARCHAR(64)  NULL,         -- notify_safe / investigation_only / hazard ...（任意）

  PRIMARY KEY (frame_id, tag_id),
  KEY idx_frame_tags_tag (tag_id),

  CONSTRAINT fk_frame_tags_frame
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Events (連続検知を束ねる：検索の主役)
-- =========================================================
CREATE TABLE IF NOT EXISTS events (
  event_id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  event_uuid       CHAR(36) NOT NULL,
  camera_id        VARCHAR(64) NOT NULL,

  start_at         DATETIME(3) NOT NULL,
  end_at           DATETIME(3) NULL,
  state            ENUM('open','closed') NOT NULL DEFAULT 'open',

  primary_event    VARCHAR(32) NOT NULL,      -- human/animal/hazard... etc
  severity_max     TINYINT UNSIGNED NOT NULL DEFAULT 0,
  confidence_max   DECIMAL(4,3) NULL,

  best_frame_id    BIGINT UNSIGNED NULL,      -- 代表フレーム
  tags_summary_json JSON NULL,                 -- 集計タグ（任意）
  retention_class  ENUM('normal','quarantine','case') NOT NULL DEFAULT 'normal',

  drive_folder_id  VARCHAR(128) NULL,         -- 隔離/案件フォルダ（必要なら）
  created_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (event_id),
  UNIQUE KEY uq_events_uuid (event_uuid),
  KEY idx_events_camera_start (camera_id, start_at),
  KEY idx_events_start (start_at),
  KEY idx_events_event_start (primary_event, start_at),

  CONSTRAINT fk_events_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE,

  CONSTRAINT fk_events_best_frame
    FOREIGN KEY (best_frame_id) REFERENCES frames(frame_id)
    ON DELETE SET NULL,

  CHECK (tags_summary_json IS NULL OR JSON_VALID(tags_summary_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS event_tags (
  event_id      BIGINT UNSIGNED NOT NULL,
  tag_id        VARCHAR(128) NOT NULL,
  tag_group     VARCHAR(64) NULL,

  PRIMARY KEY (event_id, tag_id),
  KEY idx_event_tags_tag (tag_id),

  CONSTRAINT fk_event_tags_event
    FOREIGN KEY (event_id) REFERENCES events(event_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Google Drive objects (file_id中心、URLはアプリで生成推奨)
-- =========================================================
CREATE TABLE IF NOT EXISTS drive_objects (
  drive_file_id     VARCHAR(128) NOT NULL,
  frame_id          BIGINT UNSIGNED NOT NULL,

  folder_class      ENUM('normal','quarantine','case') NOT NULL DEFAULT 'normal',
  uploaded_at       DATETIME(3) NOT NULL,
  expire_at         DATETIME(3) NULL,             -- TTL運用用

  file_name         VARCHAR(255) NULL,
  mime_type         VARCHAR(128) NULL,
  size_bytes        BIGINT UNSIGNED NULL,

  PRIMARY KEY (drive_file_id),
  KEY idx_drive_frame (frame_id),
  KEY idx_drive_expire (expire_at),

  CONSTRAINT fk_drive_objects_frame
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Vision LLM escalation logs (unknown等の確認用)
-- =========================================================
CREATE TABLE IF NOT EXISTS llm_reviews (
  review_id        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  frame_id         BIGINT UNSIGNED NULL,
  event_id         BIGINT UNSIGNED NULL,

  provider         VARCHAR(64) NOT NULL,         -- gemini/openai/etc
  model            VARCHAR(128) NULL,
  request_id       VARCHAR(128) NULL,

  status           ENUM('ok','error','skipped') NOT NULL DEFAULT 'ok',
  error_message    VARCHAR(255) NULL,

  request_json     JSON NULL,
  response_json    JSON NULL,

  created_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (review_id),
  KEY idx_llm_reviews_frame (frame_id),
  KEY idx_llm_reviews_event (event_id),
  KEY idx_llm_reviews_created (created_at),

  CONSTRAINT fk_llm_reviews_frame
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id)
    ON DELETE SET NULL,

  CONSTRAINT fk_llm_reviews_event
    FOREIGN KEY (event_id) REFERENCES events(event_id)
    ON DELETE SET NULL,

  CHECK (request_json IS NULL OR JSON_VALID(request_json)),
  CHECK (response_json IS NULL OR JSON_VALID(response_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Notifications (webhook投げた履歴)
-- =========================================================
CREATE TABLE IF NOT EXISTS notifications (
  notification_id  BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  event_id         BIGINT UNSIGNED NULL,
  frame_id         BIGINT UNSIGNED NULL,

  channel          VARCHAR(64) NOT NULL,     -- webhook/slack/etc
  status           ENUM('ok','error') NOT NULL DEFAULT 'ok',
  http_status      SMALLINT UNSIGNED NULL,
  error_message    VARCHAR(255) NULL,

  payload_json     JSON NULL,

  created_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (notification_id),
  KEY idx_notifications_created (created_at),
  KEY idx_notifications_event (event_id),
  KEY idx_notifications_frame (frame_id),

  CONSTRAINT fk_notifications_event
    FOREIGN KEY (event_id) REFERENCES events(event_id)
    ON DELETE SET NULL,

  CONSTRAINT fk_notifications_frame
    FOREIGN KEY (frame_id) REFERENCES frames(frame_id)
    ON DELETE SET NULL,

  CHECK (payload_json IS NULL OR JSON_VALID(payload_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Hourly stats (長期保持用：no_eventを間引いた後に残す)
-- =========================================================
CREATE TABLE IF NOT EXISTS hourly_stats (
  camera_id         VARCHAR(64) NOT NULL,
  hour_at           DATETIME(0) NOT NULL, -- 時間丸め（UTC推奨）

  frames_total      INT UNSIGNED NOT NULL DEFAULT 0,
  frames_analyzed   INT UNSIGNED NOT NULL DEFAULT 0,
  frames_detected   INT UNSIGNED NOT NULL DEFAULT 0,
  frames_unknown    INT UNSIGNED NOT NULL DEFAULT 0,
  offline_count     INT UNSIGNED NOT NULL DEFAULT 0,

  avg_diff_ratio    DECIMAL(9,6) NULL,

  created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (camera_id, hour_at),

  CONSTRAINT fk_hourly_stats_camera
    FOREIGN KEY (camera_id) REFERENCES cameras(camera_id)
    ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =========================================================
-- Global settings (簡易KVS)
-- =========================================================
CREATE TABLE IF NOT EXISTS settings (
  setting_key      VARCHAR(128) NOT NULL,
  setting_json     JSON NOT NULL,
  updated_at       TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (setting_key),
  CHECK (JSON_VALID(setting_json))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 初期設定例（必要なら）
INSERT IGNORE INTO settings(setting_key, setting_json) VALUES
('diff_thresholds', JSON_OBJECT(
  'diff_ratio_no_event', 0.003,
  'luma_delta_no_event', 5,
  'force_infer_every_n', 60
)),
('retention_policy', JSON_OBJECT(
  'frames_no_event_days', 14,
  'frames_detected_days', 30,
  'drive_normal_days', 14,
  'drive_quarantine_days', 60
));
```

---

# (2) schema.json（タグ辞書 初期セット）

> ポイント
>
> * `tag.id` は **返却でそのまま使う文字列**（変更しない）
> * `notify_allowed=true` のみ通知文で使用（観測事実）
> * `investigation_only` は **検索/調査画面専用**（通知に出さない）
> * is22が schema を保持し、is21へ `PUT /v1/schema` で配布

```json
{
  "schema_version": "2025-12-28.1",
  "generated_at": "2025-12-28T00:00:00Z",
  "owner": "ar-is22n(camServer)",
  "consumer": "ar-is21o(camimageEdge ai)",

  "primary_events": [
    "none",
    "human",
    "animal",
    "vehicle",
    "hazard",
    "camera_issue",
    "object_missing",
    "unknown"
  ],

  "tag_groups": [
    {
      "id": "notify_safe",
      "visibility": "default",
      "notify_allowed": true,
      "description_ja": "通知・要約で使ってよい『観測事実』タグ"
    },
    {
      "id": "investigation_only",
      "visibility": "admin_only",
      "notify_allowed": false,
      "description_ja": "調査用（検索キーとして保持）だが通知文では使わないタグ"
    }
  ],

  "tags": [
    {
      "id": "count.single",
      "group": "notify_safe",
      "label_ja": "単独",
      "severity_default": 0,
      "notify_allowed": true,
      "retention_hint": "normal",
      "description_ja": "検知対象が単独に見える"
    },
    {
      "id": "count.multiple",
      "group": "notify_safe",
      "label_ja": "複数",
      "severity_default": 1,
      "notify_allowed": true,
      "retention_hint": "normal",
      "description_ja": "検知対象が複数に見える"
    },

    { "id": "top_color.red",   "group": "notify_safe", "label_ja": "上衣:赤系",   "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "top_color.blue",  "group": "notify_safe", "label_ja": "上衣:青系",   "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "top_color.black", "group": "notify_safe", "label_ja": "上衣:黒系",   "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "top_color.white", "group": "notify_safe", "label_ja": "上衣:白系",   "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "top_color.gray",  "group": "notify_safe", "label_ja": "上衣:灰系",   "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "top_color.other", "group": "notify_safe", "label_ja": "上衣:その他", "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },

    { "id": "carry.backpack", "group": "notify_safe", "label_ja": "持ち物:リュック", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "carry.bag",      "group": "notify_safe", "label_ja": "持ち物:バッグ",   "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "carry.umbrella", "group": "notify_safe", "label_ja": "持ち物:傘",       "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },

    { "id": "appearance.hat_like",     "group": "notify_safe", "label_ja": "外観:帽子っぽい",      "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "appearance.helmet_like",  "group": "notify_safe", "label_ja": "外観:ヘルメットっぽい","severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "appearance.mask_like",    "group": "notify_safe", "label_ja": "外観:マスク/顔覆い",   "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "appearance.face_visible", "group": "investigation_only", "label_ja": "顔が見えている可能性", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },

    { "id": "behavior.loitering", "group": "notify_safe", "label_ja": "行動:滞在/うろつき", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "behavior.running",   "group": "notify_safe", "label_ja": "行動:走っている",     "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },

    { "id": "camera.blur",     "group": "notify_safe", "label_ja": "カメラ:ブレ/ボケ", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "camera.dark",     "group": "notify_safe", "label_ja": "カメラ:暗い",       "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "camera.glare",    "group": "notify_safe", "label_ja": "カメラ:白飛び/反射", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "camera.occluded", "group": "notify_safe", "label_ja": "カメラ:遮蔽/レンズ汚れ", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "camera.moved",    "group": "notify_safe", "label_ja": "カメラ:画角ズレ", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "camera.offline",  "group": "notify_safe", "label_ja": "カメラ:オフライン", "severity_default": 3, "notify_allowed": true, "retention_hint": "quarantine" },

    { "id": "hazard.smoke_like",      "group": "notify_safe", "label_ja": "異常:煙/湯気っぽい", "severity_default": 3, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "hazard.flame_like",      "group": "notify_safe", "label_ja": "異常:火っぽい",       "severity_default": 3, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "hazard.water_present",   "group": "notify_safe", "label_ja": "異常:水/浸水",         "severity_default": 3, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "hazard.water_level_high","group": "notify_safe", "label_ja": "異常:水位高い",         "severity_default": 3, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "hazard.fallen_tree_like","group": "notify_safe", "label_ja": "異常:倒木/障害物っぽい", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },

    { "id": "animal.dog",     "group": "notify_safe", "label_ja": "動物:犬", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "animal.cat",     "group": "notify_safe", "label_ja": "動物:猫", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "animal.deer",    "group": "notify_safe", "label_ja": "動物:鹿", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "animal.boar",    "group": "notify_safe", "label_ja": "動物:猪", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },
    { "id": "animal.bird",    "group": "notify_safe", "label_ja": "動物:鳥", "severity_default": 0, "notify_allowed": true, "retention_hint": "normal" },
    { "id": "animal.unknown", "group": "notify_safe", "label_ja": "動物:不明", "severity_default": 1, "notify_allowed": true, "retention_hint": "normal" },

    { "id": "outfit.suit_like",     "group": "investigation_only", "label_ja": "服装:スーツっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "outfit.kimono_like",   "group": "investigation_only", "label_ja": "服装:着物っぽい",   "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "outerwear.down_like",  "group": "investigation_only", "label_ja": "服装:ダウンっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "top.tshirt_like",      "group": "investigation_only", "label_ja": "服装:Tシャツっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "bottom.pants_like",    "group": "investigation_only", "label_ja": "服装:ズボンっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "bottom.skirt_like",    "group": "investigation_only", "label_ja": "服装:スカートっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "bottom.dress_like",    "group": "investigation_only", "label_ja": "服装:ワンピースっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "footwear.heels_like",  "group": "investigation_only", "label_ja": "履物:ヒールっぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },

    { "id": "body.size.small",   "group": "investigation_only", "label_ja": "体格:小柄っぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "body.size.medium",  "group": "investigation_only", "label_ja": "体格:中柄っぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "body.size.large",   "group": "investigation_only", "label_ja": "体格:大柄っぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "body.build.muscular_like", "group": "investigation_only", "label_ja": "体格:筋肉質っぽい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },

    { "id": "object_missing.suspected", "group": "notify_safe", "label_ja": "在庫:減少疑い", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" },

    { "id": "reason.diff_small",       "group": "investigation_only", "label_ja": "理由:差分が小さい", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "reason.low_confidence",   "group": "investigation_only", "label_ja": "理由:信頼度低い", "severity_default": 0, "notify_allowed": false, "retention_hint": "normal" },
    { "id": "reason.unknown_activity", "group": "notify_safe", "label_ja": "不明:何か起きている", "severity_default": 2, "notify_allowed": true, "retention_hint": "quarantine" }
  ],

  "guidance": {
    "notify_policy_ja": "通知・要約はnotify_allowed=trueのタグのみで文章生成する。investigation_onlyは検索/調査画面専用。",
    "retention_policy_ja": "retention_hint=quarantineのものはDrive隔離フォルダに回し保持期間を延長する。",
    "confidence_policy_ja": "confidenceが低い場合はunknown_flagを立て、必要時のみVision LLMへエスカレーションする（クールダウン必須）。"
  }
}
```

---

# (3) is22↔is21 API OpenAPI定義（OpenAPI 3.1）

> is21（camimageEdge ai）の推論APIの仕様です。
> is22（camServer）から is21 を呼ぶための定義になっています。

```yaml
openapi: 3.1.0
info:
  title: camimageEdge AI API (is21)
  version: "1.0.0"
  description: >
    Orange Pi 5 Plus (is21) の推論専用API。
    画像入力を受け、テキスト生成なしの enum/tag 形式で結果を返す。

servers:
  - url: http://is21:9000

paths:
  /healthz:
    get:
      summary: Health check
      responses:
        "200":
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/HealthResponse"

  /v1/capabilities:
    get:
      summary: Get runtime/model capabilities
      responses:
        "200":
          description: Capabilities
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/CapabilitiesResponse"

  /v1/schema:
    get:
      summary: Get current schema cached on is21
      responses:
        "200":
          description: Current schema
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SchemaEnvelope"
        "404":
          description: Schema not set
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"

    put:
      summary: Set/Update schema (pushed from is22)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/SchemaEnvelope"
      responses:
        "200":
          description: Schema accepted
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SchemaSetResponse"
        "400":
          description: Invalid schema
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"

  /v1/analyze:
    post:
      summary: Analyze one snapshot image (infer image)
      description: >
        is22からinfer用に正規化されたJPEGを受け取り推論する。
        返却は enum/tag + 数値のみ（自然文テキストは返さない）。
      requestBody:
        required: true
        content:
          multipart/form-data:
            schema:
              type: object
              required:
                - camera_id
                - captured_at
                - schema_version
                - infer_image
              properties:
                request_id:
                  type: string
                  description: Optional idempotency/request tracing id
                  examples: ["6b7f1e2a-5a2b-4f33-8cc8-2a9a39f6d9d1"]
                camera_id:
                  type: string
                  examples: ["cam_2f_19"]
                captured_at:
                  type: string
                  format: date-time
                  description: UTC推奨（is22側で統一）
                schema_version:
                  type: string
                  description: Schema version expected by is22
                  examples: ["2025-12-28.1"]

                profile:
                  type: string
                  enum: ["fast", "standard"]
                  default: "standard"
                  description: fast=軽量（最低限）、standard=通常

                return_bboxes:
                  type: boolean
                  default: true

                infer_image:
                  type: string
                  format: binary
                  description: JPEG/PNG (infer size, e.g. width 640)

                prev_infer_image:
                  type: string
                  format: binary
                  description: Optional previous infer image (if is21 should compute diff). Usually diff is computed on is22.

                hints_json:
                  type: string
                  description: >
                    Optional JSON string.
                    is22が付与したヒント（time/nightなど）を渡したい場合に使用。
                    is21は原則画像ベースの判断のみ。
                  examples:
                    - '{"time_band":"night","network_zone":"vpn"}'

      responses:
        "200":
          description: Analysis result
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/AnalyzeResponse"
        "400":
          description: Bad request / invalid image
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"
        "409":
          description: Schema version mismatch
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"
        "429":
          description: Overloaded (backpressure)
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"
        "503":
          description: Temporarily unavailable
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/ErrorResponse"

components:
  schemas:
    HealthResponse:
      type: object
      required: [status]
      properties:
        status:
          type: string
          enum: ["ok"]
        uptime_sec:
          type: integer
          minimum: 0

    CapabilitiesResponse:
      type: object
      required: [api_version, supported_primary_events]
      properties:
        api_version:
          type: string
          examples: ["1.0.0"]
        runtime:
          type: object
          properties:
            npu:
              type: string
              examples: ["rknpu2"]
            os:
              type: string
              examples: ["Armbian Ubuntu 24.04"]
        model:
          type: object
          properties:
            name:
              type: string
              examples: ["yolo-person-lite"]
            version:
              type: string
              examples: ["2025.12.28"]
        supported_primary_events:
          type: array
          items:
            type: string
          examples:
            - ["none","human","animal","vehicle","hazard","camera_issue","object_missing","unknown"]
        max_image_bytes:
          type: integer
          examples: [5000000]
        recommended_infer_widths:
          type: array
          items:
            type: integer
          examples:
            - [640, 960]

    SchemaEnvelope:
      type: object
      required: [schema_version, schema]
      properties:
        schema_version:
          type: string
          examples: ["2025-12-28.1"]
        schema:
          type: object
          description: schema.json本体（そのまま）
        received_at:
          type: string
          format: date-time

    SchemaSetResponse:
      type: object
      required: [schema_version, status]
      properties:
        schema_version:
          type: string
        status:
          type: string
          enum: ["accepted"]

    AnalyzeResponse:
      type: object
      required:
        - schema_version
        - camera_id
        - captured_at
        - analyzed
        - detected
        - primary_event
        - tags
        - severity
        - unknown_flag
      properties:
        request_id:
          type: string
          description: Echo request_id if provided
        schema_version:
          type: string
        camera_id:
          type: string
        captured_at:
          type: string
          format: date-time

        analyzed:
          type: boolean
          description: is21で推論を実施したか（is22がdiffで止める運用が基本なので通常true）

        detected:
          type: boolean
          description: 何か検知（no_eventはfalse）
        primary_event:
          type: string
          description: One of schema.primary_events
          examples: ["human"]

        tags:
          type: array
          items:
            type: string
          description: schema.tags[].id の配列（該当するものだけ）
          examples:
            - ["count.single","top_color.red","behavior.loitering"]

        confidence:
          type: number
          minimum: 0
          maximum: 1
          description: Optional confidence
        severity:
          type: integer
          minimum: 0
          maximum: 3

        unknown_flag:
          type: boolean
          description: 分類が怪しい/要確認の場合true

        count_hint:
          type: integer
          minimum: 0
          description: 人数/動物数のざっくり（任意）

        bboxes:
          type: array
          description: Normalized [x1,y1,x2,y2] in 0..1
          items:
            $ref: "#/components/schemas/BBox"

        diff:
          $ref: "#/components/schemas/DiffMetrics"

        processing_ms:
          type: integer
          minimum: 0

        model_info:
          type: object
          properties:
            name:
              type: string
            version:
              type: string

        warnings:
          type: array
          items:
            type: string

    BBox:
      type: object
      required: [x1,y1,x2,y2]
      properties:
        x1: { type: number, minimum: 0, maximum: 1 }
        y1: { type: number, minimum: 0, maximum: 1 }
        x2: { type: number, minimum: 0, maximum: 1 }
        y2: { type: number, minimum: 0, maximum: 1 }
        label:
          type: string
          description: Optional label (e.g., person)
        conf:
          type: number
          minimum: 0
          maximum: 1

    DiffMetrics:
      type: object
      properties:
        used_prev_image:
          type: boolean
        diff_ratio:
          type: number
          minimum: 0
        luma_delta:
          type: integer

    ErrorResponse:
      type: object
      required: [error, message]
      properties:
        error:
          type: string
          examples: ["bad_request", "schema_mismatch", "overloaded"]
        message:
          type: string
        details:
          type: object
```

---

## 次の実装のための「最短の整合ポイント」

* **tags検索**：`frame_tags / event_tags` を必ずINSERTする（JSONだけだと後でつらい）
* **Driveリンク**：DBには `drive_file_id` を保存し、UIで `https://drive.google.com/file/d/<file_id>/view` 形式で生成（必要ならis22がプロキシURLを発行）
* **schema運用**：`schema_versions.is_active=1` を1つに固定、is21へは起動時＋更新時にpush
