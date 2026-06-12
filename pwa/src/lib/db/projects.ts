import { getDatabase } from './index';
import type { ProjectRecord } from './schema';

export async function createProject(project: ProjectRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('projects', project);
}

export async function getProject(id: string): Promise<ProjectRecord | undefined> {
  const db = await getDatabase();
  return db.get('projects', id);
}

export async function getAllProjects(userId: string): Promise<ProjectRecord[]> {
  const db = await getDatabase();
  const all = await db.getAllFromIndex('projects', 'user_id', userId);
  return all.filter(p => !p.archived_at);
}

export async function updateProject(project: Partial<ProjectRecord> & { id: string }): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('projects', project.id);
  if (existing) {
    await db.put('projects', { ...existing, ...project });
  }
}

export async function softDeleteProject(id: string): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('projects', id);
  if (existing) {
    const now = new Date().toISOString();
    await db.put('projects', { ...existing, archived_at: now, updated_at: now });
  }
}
