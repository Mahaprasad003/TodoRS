import { writable, derived } from 'svelte/store';
import { getAllTasks, createTask as dbCreateTask, updateTask as dbUpdateTask, getTask } from '$lib/db/tasks';
import { createRecurrenceRule as dbCreateRecurrenceRule } from '$lib/db/recurrence_rules';
import type { TaskRecord, RecurrenceRuleRecord } from '$lib/db/schema';
import { generateOperation, sync, getCurrentUserId } from '$lib/sync/client';

// ── Task Store ──

export const tasksStore = writable<TaskRecord[]>([]);

// ── Derived Views ──

export const inboxTasks = derived(tasksStore, $tasks =>
  $tasks.filter(t => t.status === 'pending' && !t.deleted_at)
    .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
);

export const todayTasks = derived(tasksStore, $tasks =>
  $tasks.filter(t => t.status === 'pending' && !t.deleted_at && isToday(t.due_at))
    .sort((a, b) => {
      // Sort by priority first, then by due_at time
      const pa = priorityWeight(a.priority);
      const pb = priorityWeight(b.priority);
      if (pa !== pb) return pb - pa;
      if (a.due_at && b.due_at) return a.due_at.localeCompare(b.due_at);
      if (a.due_at) return -1;
      if (b.due_at) return 1;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    })
);

export const upcomingTasks = derived(tasksStore, $tasks =>
  $tasks.filter(t => t.status === 'pending' && !t.deleted_at && isUpcoming(t.due_at))
    .sort((a, b) => {
      if (a.due_at && b.due_at) return a.due_at.localeCompare(b.due_at);
      if (a.due_at) return -1;
      if (b.due_at) return 1;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    })
);

export const completedTodayTasks = derived(tasksStore, $tasks =>
  $tasks.filter(t => t.status === 'completed' && !t.deleted_at && isToday(t.due_at))
    .sort((a, b) => {
      const ca = a.completed_at ? new Date(a.completed_at).getTime() : new Date(a.updated_at).getTime();
      const cb = b.completed_at ? new Date(b.completed_at).getTime() : new Date(b.updated_at).getTime();
      return cb - ca;
    })
);

/** All completed (not deleted) tasks, sorted newest-first. */
export const completedTasks = derived(tasksStore, $tasks =>
  $tasks.filter(t => t.status === 'completed' && !t.deleted_at)
    .sort((a, b) => {
      const ca = a.completed_at ? new Date(a.completed_at).getTime() : new Date(a.updated_at).getTime();
      const cb = b.completed_at ? new Date(b.completed_at).getTime() : new Date(b.updated_at).getTime();
      return cb - ca;
    })
);

function priorityWeight(p: string): number {
  switch (p) {
    case 'urgent': return 4;
    case 'high': return 3;
    case 'medium': return 2;
    case 'low': return 1;
    default: return 0;
  }
}

function isToday(dateStr?: string): boolean {
  if (!dateStr) return false;
  const today = new Date();
  const todayStr = today.toISOString().slice(0, 10);
  return dateStr.slice(0, 10) === todayStr;
}

function isUpcoming(dateStr?: string): boolean {
  if (!dateStr) return false;
  const today = new Date();
  const todayStr = today.toISOString().slice(0, 10);
  return dateStr.slice(0, 10) > todayStr;
}

// ── Helpers ──

export async function loadTasks(userId: string): Promise<void> {
  const all = await getAllTasks(userId);
  tasksStore.set(all);
}

export async function addTask(title: string, options?: {
  project_id?: string;
  priority?: string;
  due_at?: string;
  recurrence?: 'none' | 'daily' | 'weekly' | 'monthly' | 'yearly';
}): Promise<void> {
  const taskId = crypto.randomUUID();
  const now = new Date().toISOString();
  const userId = getCurrentUserId() || '';

  // If recurrence is specified, create the RecurrenceRule first
  let recurrenceRuleId: string | undefined;
  if (options?.recurrence && options.recurrence !== 'none') {
    recurrenceRuleId = crypto.randomUUID();
    const rule: RecurrenceRuleRecord = {
      id: recurrenceRuleId,
      task_id: taskId,
      kind: options.recurrence,
      interval: 1,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      wait_for_completion: false,
      anchor_mode: 'schedule',
      created_at: now,
      updated_at: now,
    };
    await dbCreateRecurrenceRule(rule);
  }

  const task: TaskRecord = {
    id: taskId,
    user_id: userId,
    title,
    description: '',
    status: 'pending',
    tag_ids: [],
    priority: (options?.priority as TaskRecord['priority']) || 'none',
    due_at: options?.due_at,
    project_id: options?.project_id,
    recurrence_rule_id: recurrenceRuleId,
    created_at: now,
    updated_at: now,
  };

  // Optimistic update
  tasksStore.update(tasks => [task, ...tasks]);

  // Generate operation for sync
  await generateOperation('task', taskId, 'create', {
    title,
    description: '',
    status: 'pending',
    tag_ids: [],
    priority: options?.priority || 'none',
    due_at: options?.due_at || null,
    scheduled_at: null,
    project_id: options?.project_id || null,
    recurrence_rule_id: recurrenceRuleId || null,
  });

  // If recurrence is set, also generate a recurrence_rule create operation
  if (recurrenceRuleId && options?.recurrence && options.recurrence !== 'none') {
    await generateOperation('recurrence_rule', recurrenceRuleId, 'create', {
      task_id: taskId,
      kind: options.recurrence,
      interval: 1,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      wait_for_completion: false,
      anchor_mode: 'schedule',
    });
  }

  // Store locally
  await dbCreateTask(task);

  // Trigger background sync
  sync().then(() => {
    const userId = getCurrentUserId();
    if (userId) loadTasks(userId);
  }).catch(e => console.error('sync error after addTask:', e));
}

export async function toggleTaskComplete(taskId: string): Promise<void> {
  const now = new Date().toISOString();

  // Get current state
  const current = await getTask(taskId);
  const isCompleted = current?.status === 'completed';
  const newStatus = isCompleted ? 'pending' : 'completed';

  // Optimistic update
  tasksStore.update(tasks =>
    tasks.map(t =>
      t.id === taskId
        ? { ...t, status: newStatus, completed_at: isCompleted ? undefined : now, updated_at: now }
        : t
    )
  );

  // Update locally — MUST set both status and completed_at
  await dbUpdateTask({
    id: taskId,
    status: newStatus,
    completed_at: isCompleted ? undefined : now,
    updated_at: now,
  });

  // Generate operation
  await generateOperation('task', taskId, 'update', {
    status: newStatus,
    completed_at: isCompleted ? undefined : now,
    updated_at: now,
  });

  // Trigger background sync
  sync().then(() => {
    const userId = getCurrentUserId();
    if (userId) loadTasks(userId);
  }).catch(e => console.error('sync error after toggle:', e));
}

export async function updateTaskField(taskId: string, updates: Partial<TaskRecord>): Promise<void> {
  updates.updated_at = new Date().toISOString();

  // Optimistic update
  tasksStore.update(tasks =>
    tasks.map(t => (t.id === taskId ? { ...t, ...updates } : t))
  );

  await dbUpdateTask({ id: taskId, ...updates });

  await generateOperation('task', taskId, 'update', updates);

  sync().then(() => {
    const userId = getCurrentUserId();
    if (userId) loadTasks(userId);
  }).catch(e => console.error('sync error after updateField:', e));
}

export async function removeTask(taskId: string): Promise<void> {
  const now = new Date().toISOString();

  // Soft delete
  tasksStore.update(tasks =>
    tasks.map(t => (t.id === taskId ? { ...t, deleted_at: now, updated_at: now } : t))
  );

  await dbUpdateTask({
    id: taskId,
    deleted_at: now,
    updated_at: now,
  });

  await generateOperation('task', taskId, 'delete', {
    deleted_at: now,
    updated_at: now,
  });

  sync().then(() => {
    const userId = getCurrentUserId();
    if (userId) loadTasks(userId);
  }).catch(e => console.error('sync error after remove:', e));
}
