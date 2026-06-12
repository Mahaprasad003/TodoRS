import { writable } from 'svelte/store';

export interface SyncState {
  status: 'idle' | 'syncing' | 'synced' | 'offline' | 'error';
  lastSyncedAt: string | null;
  error: string | null;
}

export const syncStore = writable<SyncState>({
  status: 'idle',
  lastSyncedAt: null,
  error: null,
});
