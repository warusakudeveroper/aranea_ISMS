-- Migration 032: PTZ無効化フラグ追加
-- PTZ対応カメラでも操作UIを非表示にするためのフラグ
-- デフォルトはFALSE（PTZ操作可能）

ALTER TABLE cameras ADD COLUMN ptz_disabled BOOLEAN NOT NULL DEFAULT FALSE;

-- インデックスは不要（フィルタ用途ではない）
