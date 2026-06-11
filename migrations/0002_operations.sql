-- Create operations table for sync
CREATE TABLE IF NOT EXISTS operations (
    op_id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    synced_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_operations_user_device_seq ON operations(user_id, device_id, seq);
CREATE INDEX IF NOT EXISTS idx_operations_synced_at ON operations(synced_at);

-- Create sync_state table to track last synced sequence
CREATE TABLE IF NOT EXISTS sync_state (
    user_id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL,
    last_local_seq INTEGER NOT NULL DEFAULT 0,
    last_synced_seq INTEGER NOT NULL DEFAULT 0,
    last_sync_at TEXT
);
