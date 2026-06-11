-- Row Level Security policies for TodoRS sync tables
-- Run this in Supabase SQL Editor after creating the tables

-- Enable RLS on operations table
ALTER TABLE operations ENABLE ROW LEVEL SECURITY;

-- Policy: authenticated users can insert their own operations
CREATE POLICY "Users can insert their own operations"
ON operations FOR INSERT
TO authenticated
WITH CHECK (auth.uid() = user_id);

-- Policy: authenticated users can view their own operations
CREATE POLICY "Users can view their own operations"
ON operations FOR SELECT
TO authenticated
USING (auth.uid() = user_id);

-- Enable RLS on sync_state table
ALTER TABLE sync_state ENABLE ROW LEVEL SECURITY;

-- Policy: authenticated users can insert/update their own sync_state
CREATE POLICY "Users can manage their own sync_state"
ON sync_state FOR ALL
TO authenticated
USING (auth.uid() = user_id)
WITH CHECK (auth.uid() = user_id);

-- Enable RLS on snapshots table
ALTER TABLE snapshots ENABLE ROW LEVEL SECURITY;

-- Policy: authenticated users can manage their own snapshots
CREATE POLICY "Users can manage their own snapshots"
ON snapshots FOR ALL
TO authenticated
USING (auth.uid() = user_id)
WITH CHECK (auth.uid() = user_id);

-- Enable RLS on devices table
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;

-- Policy: authenticated users can manage their own devices
CREATE POLICY "Users can manage their own devices"
ON devices FOR ALL
TO authenticated
USING (auth.uid() = user_id)
WITH CHECK (auth.uid() = user_id);
