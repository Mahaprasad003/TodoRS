import { getDatabase } from './index';
import type { SyncStateRecord } from './schema';

function makeAccountKey(userId: string, deviceId: string): string {
  return `${userId}:${deviceId}`;
}

export async function getSyncState(userId: string, deviceId: string): Promise<SyncStateRecord | undefined> {
  const db = await getDatabase();
  const accountKey = makeAccountKey(userId, deviceId);
  return db.get('sync_state', accountKey);
}

export async function updateSyncState(state: SyncStateRecord): Promise<void> {
  const db = await getDatabase();
  // Ensure account_key is always populated
  const record: SyncStateRecord = {
    ...state,
    account_key: state.account_key || makeAccountKey(state.user_id, state.device_id),
  };
  await db.put('sync_state', record);
}

export async function initSyncState(userId: string, deviceId: string): Promise<void> {
  const db = await getDatabase();
  const accountKey = makeAccountKey(userId, deviceId);
  const existing = await db.get('sync_state', accountKey);
  if (!existing) {
    await db.put('sync_state', {
      account_key: accountKey,
      device_id: deviceId,
      user_id: userId,
      last_synced_at: '1970-01-01T00:00:00Z',
      last_local_seq: 0,
    });
  }
}
