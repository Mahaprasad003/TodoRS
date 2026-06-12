import { getDatabase } from './index';
import type { SyncStateRecord } from './schema';

export async function getSyncState(deviceId: string): Promise<SyncStateRecord | undefined> {
  const db = await getDatabase();
  return db.get('sync_state', deviceId);
}

export async function updateSyncState(state: SyncStateRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('sync_state', state);
}

export async function initSyncState(deviceId: string, userId: string): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('sync_state', deviceId);
  if (!existing) {
    await db.put('sync_state', {
      device_id: deviceId,
      user_id: userId,
      last_synced_at: '1970-01-01T00:00:00Z',
      last_local_seq: 0,
    });
  }
}
