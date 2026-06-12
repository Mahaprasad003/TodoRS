import { getDatabase } from './index';
import type { TagRecord } from './schema';

export async function createTag(tag: TagRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('tags', tag);
}

export async function getAllTags(userId: string): Promise<TagRecord[]> {
  const db = await getDatabase();
  const all = await db.getAllFromIndex('tags', 'user_id', userId);
  return all;
}

export async function updateTag(tag: Partial<TagRecord> & { id: string }): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('tags', tag.id);
  if (existing) {
    await db.put('tags', { ...existing, ...tag });
  }
}
