import { openDB, type IDBPDatabase } from 'idb';
import { DB_NAME, DB_VERSION, type Database } from './schema';

let dbPromise: Promise<IDBPDatabase<Database>> | null = null;

export async function getDatabase(): Promise<IDBPDatabase<Database>> {
  if (dbPromise) return dbPromise;

  dbPromise = openDB<Database>(DB_NAME, DB_VERSION, {
    upgrade(db) {
      // Tasks
      const tasksStore = db.createObjectStore('tasks', { keyPath: 'id' });
      tasksStore.createIndex('user_id', 'user_id');
      tasksStore.createIndex('status', 'status');
      tasksStore.createIndex('due_at', 'due_at');
      tasksStore.createIndex('project_id', 'project_id');
      tasksStore.createIndex('deleted_at', 'deleted_at');

      // Projects
      const projectsStore = db.createObjectStore('projects', { keyPath: 'id' });
      projectsStore.createIndex('user_id', 'user_id');
      projectsStore.createIndex('archived_at', 'archived_at');

      // Tags
      const tagsStore = db.createObjectStore('tags', { keyPath: 'id' });
      tagsStore.createIndex('user_id', 'user_id');
      tagsStore.createIndex('name', 'name');

      // Recurrence Rules
      const recurStore = db.createObjectStore('recurrence_rules', { keyPath: 'id' });
      recurStore.createIndex('task_id', 'task_id');

      // Operations
      const opsStore = db.createObjectStore('operations', { keyPath: 'op_id' });
      opsStore.createIndex('user_id', 'user_id');
      opsStore.createIndex('synced_at', 'synced_at');
      opsStore.createIndex('created_at', 'created_at');
      opsStore.createIndex('seq', 'seq');

      // Sync state (singleton per device)
      db.createObjectStore('sync_state', { keyPath: 'device_id' });

      // Metadata (key-value store for device_id, user_id, etc.)
      db.createObjectStore('metadata', { keyPath: 'key' });
    },
  });

  return dbPromise;
}
