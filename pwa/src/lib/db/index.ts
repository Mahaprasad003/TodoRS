import { openDB, type IDBPDatabase } from 'idb';
import { DB_NAME, DB_VERSION, type Database } from './schema';

let dbPromise: Promise<IDBPDatabase<Database>> | null = null;

export async function getDatabase(): Promise<IDBPDatabase<Database>> {
  if (dbPromise) return dbPromise;

  dbPromise = openDB<Database>(DB_NAME, DB_VERSION, {
    upgrade(db, oldVersion, _newVersion, _transaction) {
      // Fresh database: create all stores
      if (oldVersion === 0) {
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

        // Sync state (v2: keyed by account_key = `${user_id}:${device_id}`)
        db.createObjectStore('sync_state', { keyPath: 'account_key' });

        // Metadata (key-value store for device_id, user_id, etc.)
        db.createObjectStore('metadata', { keyPath: 'key' });
        return;
      }

      // ── v1 → v2 migration: sync_state now keyed by account_key ──
      if (oldVersion === 1) {
        // Delete old sync_state store (keyed by device_id)
        db.deleteObjectStore('sync_state');
        // Create new sync_state store keyed by account_key
        db.createObjectStore('sync_state', { keyPath: 'account_key' });
        // Entity stores (tasks, projects, operations, etc.) are preserved as-is.
        // The sync cursor is rebuildable from the server, so deleting old sync_state
        // is safe and avoids complex data migration.
      }
    },
  });

  return dbPromise;
}
