-- Chat messages storage for cross-device sync
-- Issue: Mobile view cannot load chat history (localStorage is per-device)

CREATE TABLE IF NOT EXISTS chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT NOT NULL UNIQUE,  -- Client-generated UUID
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,  -- ISO8601 format
    handled INTEGER DEFAULT 0,  -- For system messages with actions
    action_type TEXT,  -- 'preset_change' etc.
    action_camera_id TEXT,
    action_preset_id TEXT,
    dismiss_at INTEGER,  -- Unix timestamp for auto-dismiss
    is_hiding INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient retrieval
CREATE INDEX IF NOT EXISTS idx_chat_messages_timestamp ON chat_messages(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_chat_messages_role ON chat_messages(role);
