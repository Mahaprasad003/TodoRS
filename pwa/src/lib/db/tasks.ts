import { getDatabase } from './index';
import type { TaskRecord } from './schema';

export async function createTask(task: TaskRecord): Promise<void> {
  const db = await getDatabase();
  await db.put('tasks', task);
}

export async function getTask(id: string): Promise<TaskRecord | undefined> {
  const db = await getDatabase();
  return db.get('tasks', id);
}

export async function getAllTasks(userId: string): Promise<TaskRecord[]> {
  const db = await getDatabase();
  const all = await db.getAllFromIndex('tasks', 'user_id', userId);
  return all;
}

export async function updateTask(task: Partial<TaskRecord> & { id: string }): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('tasks', task.id);
  if (existing) {
    await db.put('tasks', { ...existing, ...task });
  }
}

export async function deleteTask(id: string): Promise<void> {
  const db = await getDatabase();
  await db.delete('tasks', id);
}

export async function softDeleteTask(id: string): Promise<void> {
  const db = await getDatabase();
  const existing = await db.get('tasks', id);
  if (existing) {
    const now = new Date().toISOString();
    await db.put('tasks', { ...existing, deleted_at: now, updated_at: now });
  }
}

export async function getTasksByStatus(userId: string, status: string): Promise<TaskRecord[]> {
  const db = await getDatabase();
  const all = await db.getAllFromIndex('tasks', 'user_id', userId);
  return all.filter(t => t.status === status && !t.deleted_at);
}

export async function getTasksWithDueDate(userId: string, date: string): Promise<TaskRecord[]> {
  const db = await getDatabase();
  const all = await db.getAllFromIndex('tasks', 'user_id', userId);
  return all.filter(t => {
    if (t.deleted_at) return false;
    if (t.status !== 'pending') return false;
    if (!t.due_at) return false;
    return t.due_at.startsWith(date);
  });
}
