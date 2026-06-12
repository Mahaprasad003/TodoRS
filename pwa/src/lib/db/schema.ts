// IndexedDB schema for TodoRS PWA — mirrors the TUI's SQLite schema

export const DB_NAME = 'todors-pwa';
export const DB_VERSION = 2;

// ── Record Interfaces ──

export interface TaskRecord {
  id: string;
  user_id: string;
  title: string;
  description?: string;
  status: 'pending' | 'completed';
  project_id?: string;
  tag_ids: string[];
  priority: 'none' | 'low' | 'medium' | 'high' | 'urgent';
  due_at?: string;
  scheduled_at?: string;
  recurrence_rule_id?: string;
  created_at: string;
  updated_at: string;
  completed_at?: string;
  deleted_at?: string;
}

export interface ProjectRecord {
  id: string;
  user_id: string;
  name: string;
  color?: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
  archived_at?: string;
}

export interface TagRecord {
  id: string;
  user_id: string;
  name: string;
  color?: string;
  created_at: string;
  updated_at: string;
}

export interface RecurrenceRuleRecord {
  id: string;
  task_id: string;
  kind: 'daily' | 'weekly' | 'monthly' | 'yearly';
  interval: number;
  by_weekday?: number[];
  by_monthday?: number[];
  timezone: string;
  wait_for_completion: boolean;
  anchor_mode: 'schedule' | 'completion';
  created_at: string;
  updated_at: string;
}

export interface OperationRecord {
  op_id: string;
  user_id: string;
  device_id: string;
  seq: number;
  entity: 'task' | 'project' | 'tag' | 'reminder' | 'recurrence_rule';
  entity_id: string;
  op_type: 'create' | 'update' | 'delete';
  payload: any;
  created_at: string;
  synced_at?: string;
}

export interface SyncStateRecord {
  account_key: string;
  device_id: string;
  user_id: string;
  last_synced_at: string;
  last_local_seq: number;
}

export interface MetadataRecord {
  key: string;
  value: string;
}

// ── Database Interface ──

export interface Database {
  tasks: TaskRecord;
  projects: ProjectRecord;
  tags: TagRecord;
  recurrence_rules: RecurrenceRuleRecord;
  operations: OperationRecord;
  sync_state: SyncStateRecord;
  metadata: MetadataRecord;
}
