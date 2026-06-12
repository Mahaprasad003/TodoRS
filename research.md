# Research: TodoRS PWA — Product Vision Brief

> **Role:** Steve Jobs — ruthless editor, obsession with simplicity and delight.
> **Goal:** Not a feature list. A product vision. The 5–7 moments that make a user *fall in love* with the PWA.

---

## Summary

The PWA is architecturally sound (dark theme, Linear design, solid sync, good task model) but emotionally hollow. The TUI already has `NaturalLanguageParser` and `Searching` — the PWA is missing both. The PWA feels like a checklist. It needs to feel like a **thinking partner**. The highest-leverage, lowest-effort wins are: (1) NLP quick-add, (2) search, (3) mobile swipe gestures, (4) a completed archive, and (5) a morning brief. The first two are already built in the TUI — they just need to be surfaced in the PWA.

---

## Findings

### 1. "Type Like You Talk" — NLP Quick Add

**The moment:** The user opens the app, types "call mom tomorrow at 3pm p1" in the QuickAdd bar, hits Enter. The task appears instantly with due date, time, and priority already set. No modal. No picker. No thinking.

**The mechanism:** Reuse the **existing `NaturalLanguageParser`** in `crates/todomrs-core/src/parser.rs`. It already parses `+project`, `@tag`, `p1`–`p4`, `due:expr`, `today`/`tomorrow`, time expressions (`8pm`, `14:30`), and `every day/week/month/year`. The TUI already uses this. The PWA just needs to call it from the QuickAdd bar or expose it as a global shortcut (e.g., `Cmd+K` for "new task"). The backend `addTask` function already accepts the structured fields — the PWA just needs to parse the raw text into those fields before calling.

**Why it matters more than alternatives:** The current AddTaskModal requires 5+ interactions (open modal → type title → tap project → tap date → tap priority → submit). NLP reduces this to 1. This is the #1 reason users stay with Todoist. It makes the app feel like it *understands* you.

**What it replaces:** The entire AddTaskModal becomes a secondary option. The modal is still useful for editing, but the primary entry path is the QuickAdd bar.

---

### 2. "The Magic Search" — Instant Task Finder

**The moment:** The user has 50 tasks. They remember they added "buy milk" somewhere but can't remember which project. They type `/` and a search bar appears. They type `milk`. The task appears instantly. They tap it. Done.

**The mechanism:** Client-side text search on `tasksStore`. Filter by `task.title.toLowerCase().includes(query)` or `task.description`. Show results in a floating overlay. Keyboard shortcut: `/` or `Cmd+K`. The TUI already has `Searching` mode. The PWA just needs a UI for it.

**Why it matters more than alternatives:** Without search, the app becomes unusable past ~20 tasks. The user scrolls through lists hunting for one task. Search is the difference between "I trust my app" and "I keep a second list in my head."

**What it replaces:** The need for complex filtering views. One search bar replaces 10 filter menus.

---

### 3. "Swipe to Done" — Mobile-First Gestures

**The moment:** On mobile, the user swipes right on a task. It slides off-screen with a satisfying checkmark animation. The task vanishes into the Completed section. The user feels like they're clearing a deck — not checking a box.

**The mechanism:** Add touch event handlers to `TaskItem.svelte`. Right swipe past a threshold → trigger `toggleTaskComplete`. Left swipe → trigger "reschedule to tomorrow" (or open a quick-action menu). Use CSS `transform` for smooth slide animation. The task stays visually until the animation completes, then the list re-renders.

**Why it matters more than alternatives:** The current TaskItem requires a precise tap on a 20px checkbox. On mobile, this is frustrating. Swiping is the native mobile pattern (iOS Mail, Todoist). It makes task completion feel like a game, not work. The "satisfying" micro-interaction is the dopamine hit.

**What it replaces:** The delete button (which currently shows on every item). Swipe-to-delete replaces the persistent trash icon, decluttering the UI.

---

### 4. "The Archive" — Completed View

**The moment:** The user taps a "Completed" tab. A wall of checked-off tasks appears, grouped by date: "Today: 12", "Yesterday: 8", "This week: 34". The user sees a visual wall of productivity. They feel accomplished.

**The mechanism:** New `/completed` route. Show all `status === 'completed'` tasks (not just today). Group by `completed_at` date (today, yesterday, this week, older). Show counts. The TUI already has a `Completed` view. The PWA only has a "Completed Today" section on the Today page — this is too narrow.

**Why it matters more than alternatives:** The current "Completed Today" section disappears at midnight. Users never see their progress. The Archive is the dopamine hit that makes users open the app again. It's the "I was productive today" feeling, visualized.

**What it replaces:** The empty "Completed Today" section. It gives completed tasks a permanent home, not just a sidebar.

---

### 5. "The Morning Brief" — Overdue Rescue

**The moment:** The user opens the app at 9am. Instead of seeing "No tasks due today" (depressing, because they have 3 overdue tasks from yesterday), a banner says: "Good morning. You have 3 overdue tasks from yesterday. Add them to today?" One tap reschedules all three. The user feels looked after.

**The mechanism:** When the Today view loads, check for `pending` tasks with `due_at < today`. If any exist, show a dismissible banner at the top of the Today view. A single button reschedules all overdue tasks to today (updates `due_at` to today). The TUI already tracks overdue tasks implicitly.

**Why it matters more than alternatives:** The current Today view is empty when there are overdue tasks (because they're in Inbox or past due). The user forgets them. This is the "app cares about me" moment. It's the difference between a passive list and an active assistant.

**What it replaces:** The need for the user to manually scan Inbox for overdue tasks. It automates the daily review.

---

### 6. "Focus Shield" — Single-Task Mode

**The moment:** The user taps a task, then taps a "Focus" button. The screen dims to near-black. Only the task title and a large 25-minute countdown timer remain visible. The timer ticks down. When it reaches zero, a gentle notification: "Great work! Take a break." The user feels like they actually accomplished something.

**The mechanism:** Full-screen overlay component. Start a countdown timer (25 minutes default). Use `Page Visibility API` to pause when the user switches away. Simple CSS transitions for dim/undim. No complex time tracking. The TUI already has recurrence and scheduling — the PWA just needs a simple timer overlay.

**Why it matters more than alternatives:** This is the most emotionally resonant feature in Superproductivity. The "I was productive today" feeling. The PWA currently has zero "work mode" — it's just a list. Focus mode turns the app from a *list* into a *tool*.

**What it replaces:** External Pomodoro apps. The user no longer needs a separate timer. The PWA becomes the single place they do focused work.

---

### 7. "The Streak" — Recurring Habit Flame

**The moment:** A recurring task shows "🔥 5 days" next to it. When the user completes it, the flame grows to 6. If they miss a day, the streak resets. The user feels an emotional pull to not break the chain.

**The mechanism:** For recurring tasks (already supported via `repeat` in AddTaskModal), count consecutive completions. Store the streak count in a new field or compute it from the history. Display a badge next to the task title. The TUI already has `RecurrenceEngine` — the PWA just needs to track streaks.

**Why it matters more than alternatives:** TickTick's habit tracker is what keeps users engaged. Even simple streak counts create emotional investment. The user doesn't just complete tasks — they build a habit. The app becomes a partner, not a chore.

**What it replaces:** External habit trackers. The user no longer needs a separate app for habits.

---

## What the TUI Has That the PWA Is Missing

| Feature | TUI | PWA | Gap |
|---|---|---|---|
| Natural language parser | ✅ `NaturalLanguageParser` | ❌ Raw text only | **#1 gap** |
| Search | ✅ `Searching` mode | ❌ No search | **#2 gap** |
| Completed view | ✅ `Completed` tab | ❌ Only "Completed Today" section | **#3 gap** |
| Recurring view | ✅ `Recurring` tab | ❌ No recurring view | Medium gap |
| Recurrence engine | ✅ `RecurrenceEngine` | ❌ Basic repeat only | Medium gap |
| Keyboard shortcuts | ✅ Full navigation | ❌ No shortcuts | Mobile gap |
| Focus mode | ❌ | ❌ | Both missing |
| Swipe gestures | ❌ | ❌ | Both missing |
| Streaks | ❌ | ❌ | Both missing |

---

## Sources

- **Kept:** Todoist Quick Add docs — proven NLP pattern, well-documented. Shows the exact syntax the parser should support.
- **Kept:** Superproductivity Focus Mode wiki — describes the Pomodoro/focus integration that makes the app feel like a "complete productivity system" rather than a list.
- **Kept:** Things 3 Quick Entry docs — shows the "capture from anywhere" philosophy that makes the app feel like an extension of the user's mind.
- **Kept:** TickTick Eisenhower Matrix docs — shows the "focus on what matters" approach, but more importantly, the habit tracker and streaks for emotional engagement.
- **Kept:** Todoist keyboard shortcuts — shows the `/` for search and `Q` for quick add as the primary interaction patterns.

- **Dropped:** Superproductivity Jira/GitHub integration — too niche, violates "normal person daily use" constraint.
- **Dropped:** TickTick Pomodoro timer — covered by Focus Shield above, but the timer alone is less impactful than the full-screen focus mode.
- **Dropped:** Todoist Karma/Streaks — too gamified for the minimal aesthetic. The simple streak flame is better aligned with the Linear-inspired design.

---

## Gaps

1. **NLP implementation detail:** The `NaturalLanguageParser` returns `ParsedTask` with `Option<String>` for dates. The PWA needs to map these to the actual `DateTime<Utc>` format that the database expects. The TUI already does this — the PWA can reuse the same logic.
2. **Search performance:** For 1000+ tasks, client-side search might lag. The current `tasksStore` is a full array in memory. For the initial implementation, a simple `.filter()` is sufficient. Optimization can come later.
3. **Swipe gesture complexity:** Svelte touch event handling can be tricky. The implementation needs to distinguish between tap, scroll, and swipe. A library like `svelte-gestures` or raw `touchstart`/`touchend` events are options.
4. **Streak computation:** Computing streaks from the operation history requires scanning all historical operations for a given recurring task. This could be expensive. A simple counter field on the task would be more efficient but requires a schema change.

---

## Next Steps

1. **Port `NaturalLanguageParser` to the PWA QuickAdd bar** — the biggest win. The parser already exists in `todomrs_core`. The PWA just needs a JavaScript equivalent or a WASM call.
2. **Add `/` search** — client-side filter, trivial to implement, massive impact.
3. **Implement swipe gestures** — mobile-first, feels native.
4. **Add `/completed` route** — group by date, show counts.
5. **Add morning brief banner** — check for overdue tasks on Today view load.

---

## Recommended Priority (Steve Jobs Order)

1. **NLP Quick Add** — "It just works." The core interaction.
2. **Magic Search** — "Find anything." The trust mechanism.
3. **Swipe to Done** — "It feels right." The mobile soul.
4. **The Archive** — "See what you've done." The motivation.
5. **Morning Brief** — "It knows you." The daily ritual.
6. **Focus Shield** — "Do deep work." The productivity layer.
7. **The Streak** — "Don't break the chain." The habit loop.

---
