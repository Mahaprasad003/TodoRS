import { writable, derived } from 'svelte/store';
import { tasksStore } from './tasks';
import type { TaskRecord } from '$lib/db/schema';

export const searchQuery = writable('');
export const searchOpen = writable(false);

export const searchResults = derived([tasksStore, searchQuery], ([$tasks, $query]) => {
  if (!$query.trim()) return [];
  const q = $query.toLowerCase();
  return $tasks
    .filter(t => !t.deleted_at && t.status === 'pending')
    .filter(t =>
      t.title.toLowerCase().includes(q) ||
      (t.description && t.description.toLowerCase().includes(q))
    )
    .sort((a, b) => {
      // Prioritize title starts-with over contains
      const aTitle = a.title.toLowerCase();
      const bTitle = b.title.toLowerCase();
      const aStarts = aTitle.startsWith(q);
      const bStarts = bTitle.startsWith(q);
      if (aStarts && !bStarts) return -1;
      if (!aStarts && bStarts) return 1;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    });
});

export function openSearch() {
  searchQuery.set('');
  searchOpen.set(true);
}

export function closeSearch() {
  searchOpen.set(false);
  searchQuery.set('');
}
