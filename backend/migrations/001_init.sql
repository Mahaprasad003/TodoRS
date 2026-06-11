-- Supabase backend schema for TodoRS sync
-- Run this in Supabase SQL Editor

-- Create operations table
CREATE TABLE operations (
    op_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    device_id UUID NOT NULL,
    seq BIGINT NOT NULL,
    entity TEXT NOT NULL,
    entity_id UUID NOT NULL,
    op_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    synced_at TIMESTAMPTZ,
    UNIQUE(user_id, device_id, seq)
);

-- Create index for efficient querying
CREATE INDEX idx_operations_user_device_seq ON operations(user_id, device_id, seq);
CREATE INDEX idx_operations_synced_at ON operations(synced_at);
CREATE INDEX idx_operations_user_created ON operations(user_id, created_at);

-- Create sync_state table
CREATE TABLE sync_state (
    user_id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    last_local_seq BIGINT NOT NULL DEFAULT 0,
    last_synced_seq BIGINT NOT NULL DEFAULT 0,
    last_sync_at TIMESTAMPTZ
);

-- Create snapshots table
CREATE TABLE snapshots (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    device_id UUID NOT NULL,
    snapshot_seq BIGINT NOT NULL,
    state_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_snapshots_user_device ON snapshots(user_id, device_id, snapshot_seq);

-- Create devices table for tracking
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ
);

CREATE INDEX idx_devices_user ON devices(user_id);
