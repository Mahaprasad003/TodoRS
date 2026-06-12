import { getDatabase } from './index';
import type { MetadataRecord } from './schema';

export async function getMetadata(key: string): Promise<string | undefined> {
  const db = await getDatabase();
  const record = await db.get('metadata', key);
  return record?.value;
}

export async function setMetadata(key: string, value: string): Promise<void> {
  const db = await getDatabase();
  await db.put('metadata', { key, value } as MetadataRecord);
}
