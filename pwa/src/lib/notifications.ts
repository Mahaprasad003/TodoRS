import { getMetadata, setMetadata } from '$lib/db/metadata';
import type { TaskRecord } from '$lib/db/schema';

const META_DAILY = 'last_daily_notify';
const META_NOTIFIED = 'notified_tasks';

/** Request notification permission. Call once on mount. */
export async function requestNotificationPermission(): Promise<boolean> {
  if (!('Notification' in window)) return false;
  if (Notification.permission === 'granted') return true;
  if (Notification.permission === 'denied') return false;
  const result = await Notification.requestPermission();
  return result === 'granted';
}

/**
 * Check all three notification moments after a sync completes.
 * Must be called with the current list of pending tasks.
 */
export async function checkNotifications(pendingTasks: TaskRecord[]): Promise<void> {
  if (!('Notification' in window) || Notification.permission !== 'granted') return;

  const now = new Date();
  const today = localDate(now);

  // ── 1. Morning Brief (first sync of the day) ──
  const lastBrief = await getMetadata(META_DAILY);
  if (today !== lastBrief) {
    const todayTasks = pendingTasks.filter(
      (t) => t.due_at && localDate(new Date(t.due_at)) === today
    );
    if (todayTasks.length > 0) {
      const titles = todayTasks.slice(0, 2).map((t) => t.title);
      const rest = todayTasks.length - titles.length;
      let text = 'Today: ' + titles.join(', ');
      if (rest === 1) text += ' and 1 more';
      else if (rest > 1) text += ` and ${rest} more`;
      sendNotification(text);
    }
    await setMetadata(META_DAILY, today);
  }

  // ── 2 & 3. Task due / overdue ──
  // notified_tasks maps task UUID → due_at ISO string (so we can detect reschedules)
  const raw = await getMetadata(META_NOTIFIED);
  const notified: Record<string, string> = raw ? JSON.parse(raw) : {};
  const cleaned: Record<string, string> = {};

  for (const task of pendingTasks) {
    const taskId = task.id;
    const storedDue = notified[taskId];

    if (storedDue !== undefined) {
      // Task was previously notified. Check if the due date has changed.
      // If it changed, the old notification is stale – drop it.
      // If it's the same, keep the entry to prevent re-notification.
      if (task.due_at === storedDue) {
        cleaned[taskId] = storedDue;
      }
      // If due_at changed, don't carry over the entry. The new due date
      // will trigger a fresh notification when it arrives.
      continue;
    }

    if (!task.due_at) continue;

    const dueAt = new Date(task.due_at);
    const hasTime = dueAt.getUTCHours() !== 0 || dueAt.getUTCMinutes() !== 0;

    if (hasTime) {
      // Task has a specific time — check the 5-minute "due" window
      const fiveMinAfter = new Date(dueAt.getTime() + 5 * 60 * 1000);
      if (now >= dueAt && now < fiveMinAfter) {
        // Moment 2: Task becomes due
        sendNotification('Time: ' + task.title);
        cleaned[taskId] = task.due_at;
      } else if (now >= fiveMinAfter) {
        // Moment 3: Task overdue (missed the due window)
        sendNotification(task.title + ' is overdue');
        cleaned[taskId] = task.due_at;
      }
    } else {
      // No specific time (midnight) — check if the date has passed
      if (localDate(dueAt) < today) {
        sendNotification(task.title + ' is overdue');
        cleaned[taskId] = task.due_at;
      }
      // Tasks due today without a time are covered by the morning brief
    }
  }

  await setMetadata(META_NOTIFIED, JSON.stringify(cleaned));
}

function sendNotification(body: string): void {
  try {
    new Notification('TodoRS', {
      body,
      icon: '/icon-192.png',
      badge: '/icon-192.png',
      tag: 'todors-' + Date.now(),
    });
  } catch {
    // Silently ignore if notification fails (e.g. permission revoked mid-session)
  }
}

/** Return the local date as YYYY-MM-DD. */
function localDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}
