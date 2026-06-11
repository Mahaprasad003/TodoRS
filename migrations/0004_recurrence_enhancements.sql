-- Add wait_for_completion flag to recurrence_rules
ALTER TABLE recurrence_rules ADD COLUMN wait_for_completion INTEGER NOT NULL DEFAULT 0;

-- Add anchor_mode: 'schedule' (default) or 'completion'
ALTER TABLE recurrence_rules ADD COLUMN anchor_mode TEXT NOT NULL DEFAULT 'schedule';
