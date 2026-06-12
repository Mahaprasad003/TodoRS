import { writable } from 'svelte/store';
import type { TaskRecord } from '$lib/db/schema';

/** The task currently being edited, or null if no edit is active. */
export const editingTask = writable<TaskRecord | null>(null);

export function openEdit(task: TaskRecord) {
  editingTask.set(task);
}

export function closeEdit() {
  editingTask.set(null);
}
