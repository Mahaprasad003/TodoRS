import { getNextSeq, createOperation, getUnsyncedOperations, markOperationsSynced } from '$lib/db/operations';
import { getSyncState, updateSyncState, initSyncState } from '$lib/db/sync-state';
import { getMetadata, setMetadata } from '$lib/db/metadata';
import { syncStore } from '$lib/stores/sync';
import type { OperationRecord } from '$lib/db/schema';

// ── Supabase Client ──

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY;

let accessToken: string | null = null;
let currentUserId: string | null = null;
let currentDeviceId: string | null = null;

export function setAuthToken(token: string | null) {
  accessToken = token;
}

export function setCurrentUserId(id: string | null) {
  currentUserId = id;
}

export function getCurrentUserId(): string | null {
  return currentUserId;
}

export function getDeviceId(): string | null {
  return currentDeviceId;
}

// ── Device Identity ──

export async function ensureDeviceId(): Promise<string> {
  let deviceId = await getMetadata('device_id');
  if (!deviceId) {
    deviceId = crypto.randomUUID();
    await setMetadata('device_id', deviceId);
  }
  currentDeviceId = deviceId;
  return deviceId;
}

// ── Auth ──

export async function supabaseLogin(email: string, password: string): Promise<{ access_token: string; user_id: string }> {
  const response = await fetch(`${supabaseUrl}/auth/v1/token?grant_type=password`, {
    method: 'POST',
    headers: {
      'apikey': supabaseAnonKey,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ email, password }),
  });

  if (!response.ok) {
    const err = await response.json();
    throw new Error(err.error_description || err.error || 'Login failed');
  }

  const data = await response.json();
  accessToken = data.access_token;
  currentUserId = data.user?.id;

  return { access_token: data.access_token, user_id: data.user?.id };
}

// ── Operation Generation ──

export async function generateOperation(
  entity: 'task' | 'project' | 'tag' | 'recurrence_rule',
  entityId: string,
  opType: 'create' | 'update' | 'delete',
  payload: any
): Promise<OperationRecord> {
  const deviceId = await ensureDeviceId();
  if (!currentUserId) throw new Error('Not authenticated');
  const seq = await getNextSeq(currentUserId, deviceId);

  // Wrap payload in the serde externally-tagged enum format the TUI expects.
  // Rust enum variants like TaskCreate { title } serialize as { "task_create": { "title": ... } }
  // Unit variants like Delete serialize as just the string "delete".
  let wrappedPayload: any;
  if (opType === 'delete') {
    // Unit variant — serde serializes as a bare string
    wrappedPayload = 'delete';
  } else {
    const variantKey = `${entity}_${opType}`; // e.g. "task_create", "project_update", "recurrence_rule_create"
    wrappedPayload = { [variantKey]: payload };
  }

  const op: OperationRecord = {
    op_id: crypto.randomUUID(),
    user_id: currentUserId,
    device_id: deviceId,
    seq,
    entity,
    entity_id: entityId,
    op_type: opType,
    payload: wrappedPayload,
    created_at: new Date().toISOString(),
  };

  await createOperation(op);
  return op;
}

// ── Upload / Download ──

async function uploadOperations(operations: OperationRecord[]): Promise<void> {
  if (!accessToken) throw new Error('Not authenticated');

  const response = await fetch(`${supabaseUrl}/functions/v1/upload-operations`, {
    method: 'POST',
    headers: {
      'apikey': supabaseAnonKey,
      'Authorization': `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ operations }),
  });

  if (!response.ok) {
    const body = await response.text().catch(() => '(no body)');
    console.error('upload-operations failed:', response.status, body);
    throw new Error(`Upload failed (${response.status}): ${body}`);
  }
}

async function getRemoteOperations(since: string): Promise<any[]> {
  if (!accessToken) throw new Error('Not authenticated');

  const url = `${supabaseUrl}/functions/v1/get-operations?since=${encodeURIComponent(since)}`;
  const response = await fetch(url, {
    method: 'GET',
    headers: {
      'apikey': supabaseAnonKey,
      'Authorization': `Bearer ${accessToken}`,
    },
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({}));
    throw new Error(err.error || `Get operations failed (${response.status})`);
  }

  const data = await response.json();
  return data.operations || [];
}

// ── Apply Remote Operations ──

import { createTask, updateTask as updateTaskInDb, softDeleteTask, getTask } from '$lib/db/tasks';
import { createProject, updateProject as updateProjectInDb, softDeleteProject, getAllProjects } from '$lib/db/projects';

/**
 * Normalize an operation payload that may come in serde externally-tagged enum format (TUI)
 * or flat format (PWA).
 *
 * TUI format: { "task_create": { "title": "...", ... } }  (variant key wrapping fields)
 * PWA format: { "title": "...", ... }                       (flat, no wrapping)
 * Delete TUI: "delete"                                         (serde unit variant -> JSON string)
 */
function unwrapPayload(op: any): any {
  if (typeof op.payload === 'string') {
    // Unit variant (e.g. serde "delete" for Delete enum variant)
    return {};
  }
  if (!op.payload || typeof op.payload !== 'object') {
    return op.payload || {};
  }

  // Check for serde externally-tagged variant keys
  const variantKeys = [
    `${op.entity}_${op.op_type}`,     // "task_create", "task_update", "project_create", etc.
    `${op.entity}_${op.op_type}s`,    // edge case: "task_updates"? no, but safe
  ];

  for (const key of variantKeys) {
    if (op.payload[key] && typeof op.payload[key] === 'object') {
      return op.payload[key];
    }
  }

  // Check for known aliases (entity + op_type doesn't always match enums)
  if (op.op_type === 'create' && op.payload.task_create) return op.payload.task_create;
  if (op.op_type === 'create' && op.payload.project_create) return op.payload.project_create;
  if (op.op_type === 'create' && op.payload.tag_create) return op.payload.tag_create;
  if (op.op_type === 'update' && op.payload.task_update) return op.payload.task_update;
  if (op.op_type === 'update' && op.payload.project_update) return op.payload.project_update;

  // Flat format (PWA-generated ops)
  return op.payload;
}

async function applyRemoteOperation(op: any): Promise<void> {
  try {
    const p = unwrapPayload(op);

    switch (op.entity) {
      case 'task': {
        switch (op.op_type) {
          case 'create': {
            // Always apply create — createTask uses db.put() which upserts.
            // This ensures stale/Untitled cached data gets corrected on re-sync.
            await createTask({
              id: op.entity_id,
              user_id: op.user_id,
              title: p.title || 'Untitled',
              description: p.description || '',
              status: p.status || 'pending',
              tag_ids: p.tag_ids || [],
              priority: p.priority || 'none',
              due_at: p.due_at || null,
              scheduled_at: p.scheduled_at || null,
              project_id: p.project_id || null,
              recurrence_rule_id: p.recurrence_rule_id || null,
              created_at: p.created_at || op.created_at,
              updated_at: p.updated_at || op.created_at,
              completed_at: p.completed_at || null,
              deleted_at: p.deleted_at || null,
            });
            break;
          }
          case 'update': {
            const existing = await getTask(op.entity_id);
            if (existing) {
              await updateTaskInDb({
                id: op.entity_id,
                ...p,
              });
            } else {
              // Create if doesn't exist (race condition - remote created first)
              await createTask({
                id: op.entity_id,
                user_id: op.user_id,
                title: p.title || 'Untitled',
                status: 'pending',
                tag_ids: [],
                priority: 'none',
                created_at: op.created_at,
                updated_at: op.created_at,
                ...p,
              });
            }
            break;
          }
          case 'delete': {
            await softDeleteTask(op.entity_id);
            break;
          }
        }
        break;
      }

      case 'project': {
        switch (op.op_type) {
          case 'create': {
            // Dedup by name: skip if a project with the same name already exists
            // (TUI can create duplicates with different IDs; keep the first one)
            const existing = await getAllProjects(op.user_id);
            if (p.name && existing.some(proj => proj.name === p.name)) {
              break;
            }
            await createProject({
              id: op.entity_id,
              user_id: op.user_id,
              name: p.name || 'Untitled',
              color: p.color || null,
              sort_order: p.sort_order || 0,
              created_at: p.created_at || op.created_at,
              updated_at: p.updated_at || op.created_at,
              archived_at: p.archived_at || null,
            });
            break;
          }
          case 'update': {
            await updateProjectInDb({
              id: op.entity_id,
              ...p,
            });
            break;
          }
          case 'delete': {
            await softDeleteProject(op.entity_id);
            break;
          }
        }
        break;
      }

      // Tags are stored but not applied during remote sync (matching TUI behavior)
      case 'tag':
      case 'reminder':
      case 'recurrence_rule':
        // Skipped — Phase 10 handles tag sync and recurrence
        break;
    }
  } catch (err) {
    console.error('Error applying remote operation:', op.op_id, err);
  }
}

// ── Sync queue (one at a time) ──

let syncPromise: Promise<SyncResult> | null = null;

async function syncRunner(): Promise<SyncResult> {
  if (!accessToken || !currentUserId) {
    syncStore.set({ status: 'error', lastSyncedAt: null, error: 'Not authenticated' });
    return { uploaded: 0, applied: 0 };
  }

  const deviceId = await ensureDeviceId();

  syncStore.set({ status: 'syncing', lastSyncedAt: null, error: null });

  try {
    // 1. Upload unsynced operations
    const unsyncedOps = await getUnsyncedOperations();
    if (unsyncedOps.length > 0) {
      await uploadOperations(unsyncedOps);
      await markOperationsSynced(unsyncedOps.map(op => op.op_id));
    }

    // 2. Initialize sync state if needed
    await initSyncState(deviceId, currentUserId);

    // 3. Download remote operations after last_synced_at
    const syncState = await getSyncState(deviceId);
    const since = syncState?.last_synced_at || '1970-01-01T00:00:00Z';
    const remoteOps = await getRemoteOperations(since);

    // 4. Apply remote operations
    let appliedCount = 0;
    let newestTime = since;

    for (const op of remoteOps) {
      if (op.created_at > newestTime) {
        newestTime = op.created_at;
      }
      if (op.device_id === deviceId || op.user_id !== currentUserId) {
        continue;
      }
      await applyRemoteOperation(op);
      appliedCount++;
    }

    // 5. Update sync state
    await updateSyncState({
      device_id: deviceId,
      user_id: currentUserId,
      last_synced_at: newestTime,
      last_local_seq: syncState?.last_local_seq || 0,
    });

    syncStore.set({ status: 'synced', lastSyncedAt: new Date().toISOString(), error: null });

    return { uploaded: unsyncedOps.length, applied: appliedCount };
  } catch (err: any) {
    const errMsg = err.message || 'Sync failed';
    syncStore.set({ status: 'error', lastSyncedAt: null, error: errMsg });
    return { uploaded: 0, applied: 0 };
  }
}

// ── Main Sync Function (queued) ──

export interface SyncResult {
  uploaded: number;
  applied: number;
}

export async function sync(): Promise<SyncResult> {
  // Wait for any in-flight sync to finish, then run a fresh one
  while (syncPromise) {
    try { await syncPromise; } catch { /* ignore previous errors */ }
  }
  syncPromise = syncRunner();
  try {
    return await syncPromise;
  } finally {
    syncPromise = null;
  }
}

// ── Sync with Retry ──

export async function syncWithRetry(maxRetries = 3): Promise<SyncResult> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await sync();
    } catch (e) {
      if (attempt === maxRetries) throw e;
    }
    // Wait before retry (exponential backoff)
    await new Promise(r => setTimeout(r, Math.min(1000 * Math.pow(2, attempt - 1), 5000)));
  }
  return { uploaded: 0, applied: 0 };
}

// ── Bootstrap ──

export async function bootstrapAfterAuth(userId: string, token: string): Promise<void> {
  setAuthToken(token);
  setCurrentUserId(userId);
  const deviceId = await ensureDeviceId();
  await initSyncState(deviceId, userId);
  await syncWithRetry();
}
