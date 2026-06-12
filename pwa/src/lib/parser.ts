/**
 * Natural language task parser — ported from Rust `NaturalLanguageParser`.
 *
 * Recognised markers:
 * - `+project` → project
 * - `@tag`     → tags (multiple allowed)
 * - `p1`–`p4`  → priority (p1=Urgent … p4=Low)
 * - `due:expr` → due_date (value after colon)
 * - `today`, `tomorrow`, weekday names → due_date
 * - `8pm`, `9am`, `14:30` → due_time
 * - `every day/week/month/year [N …]` → recurrence
 * - `wait!` → wait_for_completion (before `every`)
 * - `every!` → completion-anchored recurrence
 */

export interface ParsedTask {
  title: string;
  project: string | null;
  tags: string[];
  priority: 'none' | 'low' | 'medium' | 'high' | 'urgent';
  dueDate: string | null;
  dueTime: string | null;
  recurrence: string | null;
  waitForCompletion: boolean;
  anchorMode: 'schedule' | 'completion';
}

const PERIOD_WORDS = new Set([
  'day', 'days', 'week', 'weeks', 'month', 'months', 'year', 'years',
]);

const DATE_WORDS = new Set([
  'today', 'tomorrow',
  'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday',
]);

function isPeriodWord(word: string): boolean {
  return PERIOD_WORDS.has(word.toLowerCase());
}

function isDateWord(word: string): boolean {
  return DATE_WORDS.has(word.toLowerCase());
}

function isTimeWord(word: string): boolean {
  const lower = word.toLowerCase();

  // pattern: <digits>am or <digits>pm
  if (lower.length >= 2) {
    const suffix = lower.slice(-2);
    if (suffix === 'am' || suffix === 'pm') {
      const numStr = lower.slice(0, -2);
      const hour = parseInt(numStr, 10);
      if (!isNaN(hour) && hour >= 1 && hour <= 12) return true;
    }
  }

  // pattern: HH:MM (24-hour)
  const colonPos = lower.indexOf(':');
  if (colonPos > 0) {
    const h = parseInt(lower.slice(0, colonPos), 10);
    const m = parseInt(lower.slice(colonPos + 1), 10);
    if (!isNaN(h) && !isNaN(m) && h >= 0 && h < 24 && m >= 0 && m < 60) return true;
  }

  return false;
}

export function parse(input: string): ParsedTask {
  const titleParts: string[] = [];
  let project: string | null = null;
  const tags: string[] = [];
  let priority: 'none' | 'low' | 'medium' | 'high' | 'urgent' = 'none';
  let dueDate: string | null = null;
  let dueTime: string | null = null;
  let recurrence: string | null = null;
  let waitForCompletion = false;
  let anchorMode: 'schedule' | 'completion' = 'schedule';

  const words = input.trim().split(/\s+/).filter(Boolean);
  let i = 0;

  while (i < words.length) {
    const word = words[i];

    // ── wait! prefix ──
    if (word.toLowerCase() === 'wait!') {
      waitForCompletion = true;
      i++;
      continue;
    }

    // ── due:prefix ──
    const colonPos = word.indexOf(':');
    if (colonPos > 0 && word.slice(0, colonPos).toLowerCase() === 'due') {
      const rest = word.slice(colonPos + 1);
      if (rest.length > 0) dueDate = rest;
      i++;
      continue;
    }

    // ── +project ──
    if (word.startsWith('+') && word.length > 1) {
      project = word.slice(1);
      i++;
      continue;
    }

    // ── @tag ──
    if (word.startsWith('@') && word.length > 1) {
      tags.push(word.slice(1));
      i++;
      continue;
    }

    // ── priority p1–p4 ──
    if (word.length === 2 && word[0].toLowerCase() === 'p') {
      const p = word.toLowerCase();
      if (p === 'p1') { priority = 'urgent'; i++; continue; }
      if (p === 'p2') { priority = 'high'; i++; continue; }
      if (p === 'p3') { priority = 'medium'; i++; continue; }
      if (p === 'p4') { priority = 'low'; i++; continue; }
      // p0, p5, … fall through to title
    }

    // ── every! … recurrence (completion-anchored) ──
    if (word.toLowerCase() === 'every!' && i + 1 < words.length) {
      anchorMode = 'completion';
      const next = words[i + 1];
      if (isPeriodWord(next)) {
        recurrence = `every ${next}`;
        i += 2;
        continue;
      }
      // every! N days/weeks/months/years
      if (i + 2 < words.length) {
        const n = parseInt(next, 10);
        if (!isNaN(n) && n > 0 && isPeriodWord(words[i + 2])) {
          recurrence = `every ${next} ${words[i + 2]}`;
          i += 3;
          continue;
        }
      }
      // fall through
    }

    // ── every … recurrence ──
    if (word.toLowerCase() === 'every' && i + 1 < words.length) {
      anchorMode = 'schedule';
      const next = words[i + 1];
      if (isPeriodWord(next)) {
        recurrence = `every ${next}`;
        i += 2;
        continue;
      }
      // every N days/weeks/months/years
      if (i + 2 < words.length) {
        const n = parseInt(next, 10);
        if (!isNaN(n) && n > 0 && isPeriodWord(words[i + 2])) {
          recurrence = `every ${next} ${words[i + 2]}`;
          i += 3;
          continue;
        }
      }
    }

    // ── raw date words ──
    if (isDateWord(word)) {
      dueDate = word.toLowerCase();
      i++;
      continue;
    }

    // ── time expressions ──
    if (isTimeWord(word)) {
      dueTime = word.toLowerCase();
      i++;
      continue;
    }

    // ── everything else → title ──
    titleParts.push(word);
    i++;
  }

  return {
    title: titleParts.join(' '),
    project,
    tags,
    priority,
    dueDate,
    dueTime,
    recurrence,
    waitForCompletion,
    anchorMode,
  };
}

/** Resolve a date word to a date string (YYYY-MM-DD). */
export function resolveDate(dueDate: string | null): string | null {
  if (!dueDate) return null;
  const today = new Date();

  switch (dueDate.toLowerCase()) {
    case 'today':
      return toDateString(today);
    case 'tomorrow': {
      const d = new Date(today);
      d.setDate(d.getDate() + 1);
      return toDateString(d);
    }
    default: {
      const weekday = parseWeekday(dueDate);
      if (weekday === null) return null;
      return toDateString(nextWeekday(today, weekday));
    }
  }
}

/** Resolve a time expression to HH:MM string. */
export function resolveTime(dueTime: string | null): string | null {
  if (!dueTime) return null;
  const lower = dueTime.toLowerCase();

  // <digits>am / <digits>pm
  if (lower.length >= 2) {
    const suffix = lower.slice(-2);
    if (suffix === 'am' || suffix === 'pm') {
      const numStr = lower.slice(0, -2);
      const hour = parseInt(numStr, 10);
      if (!isNaN(hour) && hour >= 1 && hour <= 12) {
        const h24 = suffix === 'am'
          ? (hour === 12 ? 0 : hour)
          : (hour === 12 ? 12 : hour + 12);
        return `${String(h24).padStart(2, '0')}:00`;
      }
    }
  }

  // HH:MM 24-hour
  const colonPos = lower.indexOf(':');
  if (colonPos > 0) {
    const h = parseInt(lower.slice(0, colonPos), 10);
    const m = parseInt(lower.slice(colonPos + 1), 10);
    if (!isNaN(h) && !isNaN(m) && h >= 0 && h < 24 && m >= 0 && m < 60) {
      return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
    }
  }

  return null;
}

/** Combine resolved date and time into an ISO datetime string (or null).
 *  Dates/times are interpreted as LOCAL and converted to UTC for storage. */
export function resolveDatetime(dueDate: string | null, dueTime: string | null): string | null {
  const date = resolveDate(dueDate);
  const time = resolveTime(dueTime);

  if (date && time) {
    const [y, m, d] = date.split('-').map(Number);
    const [h, min] = time.split(':').map(Number);
    return new Date(y, m - 1, d, h, min).toISOString();
  }
  if (date) {
    const [y, m, d] = date.split('-').map(Number);
    return new Date(y, m - 1, d).toISOString();
  }
  if (time) {
    const now = new Date();
    const [h, min] = time.split(':').map(Number);
    now.setHours(h, min, 0, 0);
    return now.toISOString();
  }
  return null;
}

/** Resolve recurrence string to { kind, interval } or null. */
export function resolveRecurrence(recurrence: string | null, anchorMode: 'schedule' | 'completion', waitForCompletion: boolean): {
  kind: 'daily' | 'weekly' | 'monthly' | 'yearly';
  interval: number;
  waitForCompletion: boolean;
  anchorMode: 'schedule' | 'completion';
} | null {
  if (!recurrence) return null;
  const words = recurrence.split(/\s+/);
  if (words.length < 2 || words[0].toLowerCase() !== 'every') return null;

  let interval = 1;
  let period: string;

  if (words.length === 2) {
    period = words[1];
  } else if (words.length === 3) {
    const n = parseInt(words[1], 10);
    if (isNaN(n) || n <= 0) return null;
    interval = n;
    period = words[2];
  } else {
    return null;
  }

  let kind: 'daily' | 'weekly' | 'monthly' | 'yearly';
  switch (period.toLowerCase()) {
    case 'day': case 'days': kind = 'daily'; break;
    case 'week': case 'weeks': kind = 'weekly'; break;
    case 'month': case 'months': kind = 'monthly'; break;
    case 'year': case 'years': kind = 'yearly'; break;
    default: return null;
  }

  return { kind, interval, waitForCompletion, anchorMode };
}

// ── Helpers ──

function toDateString(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function parseWeekday(s: string): number | null {
  const map: Record<string, number> = {
    monday: 1, tuesday: 2, wednesday: 3, thursday: 4,
    friday: 5, saturday: 6, sunday: 0,
  };
  return map[s.toLowerCase()] ?? null;
}

/** Return the next occurrence of `weekday` on or after `from`.
 *  If `from` is already the target weekday, returns 7 days later. */
function nextWeekday(from: Date, targetDay: number): Date {
  const fromDay = from.getDay(); // 0=Sun
  if (targetDay === fromDay) {
    const d = new Date(from);
    d.setDate(d.getDate() + 7);
    return d;
  }
  const daysAhead = (targetDay - fromDay + 7) % 7;
  const d = new Date(from);
  d.setDate(d.getDate() + daysAhead);
  return d;
}
