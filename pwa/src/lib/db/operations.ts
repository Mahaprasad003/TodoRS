import { getDatabase } from './index';
import type { OperationRecord } from './schema';

export async function createOperation(op: OperationRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('operations', op);
}

export async function getUnsyncedOperations(): Promise<OperationRecord[]> {
  const db = await getDatabase();
  const all = await db.getAll('operations');
  return all.filter(op => !op.synced_at).sort((a, b) => a.seq - b.seq);
}

export async function getAllOperations(): Promise<OperationRecord[]> {
  const db = await getDatabase();
  return db.getAll('operations');
}

export async function markOperationsSynced(opIds: string[]): Promise<void> {
  const db = await getDatabase();
  const now = new Date().toISOString();
  for (const opId of opIds) {
    const op = await db.get('operations', opId);
    if (op) {
      op.synced_at = now;
      await db.put('operations', op);
    }
  }
}

export async function getNextSeq(userId: string, deviceId: string): Promise<number> {
  const db = await getDatabase();
  const ops = await db.getAllFromIndex('operations', 'user_id', userId);
  const maxSeq = ops.filter(op => op.device_id === deviceId).reduce((max, op) => Math.max(max, op.seq), 0);
  return maxSeq + 1;
}
