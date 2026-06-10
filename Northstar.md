# TodoRS Northstar

## One-line product vision

TodoRS is a fast, reliable, personal task manager with a beautiful keyboard-first TUI, a mobile-friendly PWA, automatic sync across devices, offline tolerance, recurring tasks, reminders, and zero mandatory paid infrastructure.

It is not a Todoist clone. It is not a todo.txt wrapper. It is a personal productivity system optimized for speed, ownership, and daily reliability.

---

## What TodoRS must become

TodoRS should feel like this:

- Open the TUI.
- Type a task naturally: `pay electricity bill tomorrow 8pm +personal p1`.
- It appears immediately.
- It syncs to mobile automatically.
- The mobile PWA can show Today, Inbox, Projects, and reminders.
- If the network is down, both TUI and PWA still work locally.
- When network returns, changes sync without data loss.

The product should be excellent for one person managing personal, academic, and professional tasks across desktop and phone.

---

## Hard requirements

### 1. Excellent TUI experience

The TUI is the primary power-user interface.

It must be:

- fast to launch
- keyboard-first
- visually clean
- efficient for capture, planning, editing, and completing tasks
- pleasant enough to use every day

Core TUI views:

- Inbox
- Today
- Upcoming
- Projects
- Search
- Completed / Archive

Core TUI actions:

- add task
- natural language quick add
- edit task
- complete / uncomplete task
- delete / archive task
- reschedule task
- assign project
- assign tags
- change priority
- sync now

Calendar and Kanban are not core. They may come later, but should not shape the initial architecture.

---

### 2. Mobile compatibility through a first-party PWA

Mobile compatibility is not optional.

TodoRS should not depend on third-party todo.txt apps for mobile support. The correct mobile path is a first-party PWA that uses the same task model and sync protocol as the TUI.

The PWA should support:

- Inbox
- Today
- Projects
- quick add
- edit task
- complete task
- recurring tasks
- reminders / notifications where platform support allows
- offline use through IndexedDB
- background sync where browser/platform support allows

Native mobile apps may come later, but the PWA is the first mobile target.

---

### 3. Automatic sync across devices

TodoRS must sync automatically between:

- desktop TUI
- mobile PWA
- future desktop/web clients

Important distinction:

- SQLite is acceptable as local storage.
- Syncing the raw SQLite database file is not acceptable.

Each client keeps a local database/cache, but sync happens through small operations, not by uploading database files.

Example operations:

```txt
CREATE_TASK
UPDATE_TASK_TITLE
COMPLETE_TASK
RESCHEDULE_TASK
MOVE_TASK_TO_PROJECT
DELETE_TASK
CREATE_PROJECT
```

The sync system should exchange operations between clients and backend.

---

### 4. Offline-tolerant, not file-only local-first

TodoRS should work offline, but the intended product experience is automatic cloud sync.

Offline behavior:

- local changes save instantly
- user can continue working without internet
- operations queue locally
- when online, queued operations sync
- remote changes are pulled and applied

This means TodoRS is local-capable, but not limited to a local-file workflow.

---

### 5. Free by default

The user wants the system to be completely free to run initially.

Therefore:

- avoid mandatory paid services
- use free-tier-friendly architecture
- avoid heavy always-on infrastructure where possible
- design backend so it can move providers later
- do not rely on fragile implementation details like pinging a sleeping service as the core architecture

Acceptable initial backend directions:

- Supabase free tier
- Firebase free tier
- Cloudflare Workers / D1 / KV where suitable
- other free-tier backend-as-a-service options
- lightweight self-hostable backend later

Reality check:

Free tiers are useful, but not guaranteed production infrastructure. TodoRS should start free, but its architecture must remain portable if the free backend becomes limiting.

---

## Non-goals for early versions

Do not build these early:

- team collaboration
- comments
- file attachments
- Kanban boards
- calendar layout
- complex analytics
- heavy time tracking
- Pomodoro
- AI task assistant
- enterprise permissions
- native mobile apps
- full Todoist parity
- full Super Productivity parity

These features are distractions until the core loop is excellent.

The core loop is:

```txt
capture → plan → execute → sync → remind
```

---

## Canonical architecture

TodoRS should use a real structured task model. todo.txt must not be the source of truth.

### Canonical model

The canonical model should include:

```txt
User
Device
Task
Project
Tag
Reminder
RecurrenceRule
Operation
SyncState
```

### Task fields

A task should support at least:

```txt
id
user_id
title
description
status
project_id
tag_ids
priority
due_at
scheduled_at
recurrence_rule_id
created_at
updated_at
completed_at
deleted_at
```

### Project fields

```txt
id
user_id
name
color
sort_order
created_at
updated_at
archived_at
```

### Tag fields

```txt
id
user_id
name
color
created_at
updated_at
```

### Reminder fields

```txt
id
task_id
remind_at
status
created_at
updated_at
```

### Recurrence fields

```txt
id
task_id
kind
interval
by_weekday
by_monthday
timezone
created_at
updated_at
```

Recurrence must be treated carefully. It is a core feature, not an afterthought.

---

## Client storage

### TUI local storage

Use SQLite.

Why:

- reliable
- fast
- easy querying
- good for filters/views/search
- durable offline cache

Do not sync the SQLite file directly.

### PWA local storage

Use IndexedDB.

Why:

- browser-native
- works offline
- appropriate for PWA storage
- can cache tasks, projects, tags, operations, and sync state

### Shared schema

The Rust TUI and TypeScript PWA must use the same logical model.

Prefer a versioned schema contract:

- JSON Schema, or
- OpenAPI types, or
- shared manually-maintained schema docs with tests

Every schema change needs migration logic.

---

## Sync architecture

The sync system should be operation-based.

Each user/device action creates an operation.

Example operation:

```json
{
  "op_id": "01JYEXAMPLE",
  "user_id": "user_123",
  "device_id": "device_desktop",
  "seq": 42,
  "entity": "task",
  "entity_id": "task_abc",
  "type": "task.update",
  "payload": {
    "title": "Submit assignment"
  },
  "created_at": "2026-06-10T12:00:00Z"
}
```

### Why operation sync

Operation sync is required because:

- TUI and PWA use different local databases
- raw SQLite sync is unsafe
- offline edits need to merge later
- reminders and recurrence need consistent semantics
- future clients need a stable protocol

### Basic sync flow

1. Client performs local action.
2. Client writes change to local DB immediately.
3. Client appends operation to local operation queue.
4. Client uploads unsynced operations to backend.
5. Backend stores operations.
6. Other clients pull missing operations.
7. Other clients apply operations to their local DB.

---

## Conflict handling

Early conflict policy should be simple but safe.

Rules:

- Different fields changed on same task: merge both.
- Same field changed on same task: deterministic latest-write-wins initially.
- Deletion vs update: do not silently lose data; preserve enough information to recover.
- Duplicate operation IDs: ignore duplicate.
- Operation application must be idempotent.

The app must prefer duplicate/preserved data over silent loss.

Vector clocks may be added later if conflict handling becomes insufficient. The operation schema should not block that future path.

---

## Backend responsibilities

The backend exists to sync clients and support notifications.

Core backend responsibilities:

- authenticate users
- register devices
- store operations
- return missing operations to clients
- store current task snapshots if needed
- store reminder schedule
- support push notification workflow for PWA

The backend should not be a heavy monolith.

Initial acceptable backend shape:

```txt
Auth
Operations API
Tasks snapshot API if useful
Reminder scheduler / notification worker
```

The backend must be designed so it can be hosted on a free tier initially and moved later.

---

## Notifications

Notifications are part of the vision, but they require platform-specific handling.

### Desktop/TUI notifications

Likely path:

- local background daemon
- reads local SQLite
- checks reminder times
- emits OS notifications
- syncs periodically

This can come after core TUI + sync.

### Mobile/PWA notifications

Likely path:

- PWA registers for Web Push
- backend stores push subscription
- backend sends push notification when reminder is due

Caveat:

PWA notification support differs by platform/browser. The implementation must verify browser support and degrade gracefully.

---

## Natural language quick add

Natural language quick add is a core feature.

It should parse common inputs like:

```txt
pay electricity bill tomorrow 8pm +personal p1
submit report every monday +work
call bank friday @phone
review notes next week +vit
```

Initial parser should be deterministic, not LLM-based.

It should extract:

- title
- due date
- due time
- recurrence
- project
- tags
- priority

The parser must be predictable. If uncertain, preserve the original title rather than over-parsing incorrectly.

---

## Recurring task semantics

Recurring tasks are core and must be implemented deliberately.

The system must define behavior for:

- daily recurrence
- weekly recurrence
- monthly recurrence
- completion of overdue recurring task
- completion before due date
- timezone handling
- whether recurrence advances from due date or completion date

Initial recommendation:

- recurrence advances from scheduled due date, not completion date
- if a recurring task is overdue and completed, generate the next future due date
- preserve completion history
- do not mutate history destructively

---

## todo.txt role

todo.txt is not canonical.

TodoRS may support:

- todo.txt import
- todo.txt export
- periodic backup export

But TodoRS must not be limited by todo.txt.

Reason:

- todo.txt is poor for recurrence, reminders, IDs, rich notes, and sync conflicts
- mobile support will come from the first-party PWA, not third-party todo.txt apps

---

## Suggested implementation stack

### TUI

```txt
Rust
ratatui
crossterm
SQLite via sqlx or rusqlite
serde
chrono or jiff
clap for CLI helpers
```

### PWA

```txt
TypeScript
SvelteKit or React
IndexedDB wrapper
Service worker
Web Push support
```

SvelteKit is attractive for small, fast applications. React is acceptable if the coding agent is stronger with React.

### Backend

Possible options:

```txt
Supabase
Firebase
Cloudflare Workers + D1/KV
Node/Fastify + hosted Postgres
Rust/Axum + hosted Postgres
```

For zero-cost MVP, prefer a managed free-tier backend over custom server ops unless the implementation plan explicitly justifies otherwise.

---

## Product quality bar

TodoRS should not be judged by number of features.

It should be judged by:

- capture speed
- sync reliability
- mobile usability
- recurrence correctness
- no data loss
- TUI smoothness
- predictable behavior
- simple mental model

A small reliable product is better than a large fragile one.

---

## Initial milestone definition

The first meaningful milestone is not “all features done.”

The first meaningful milestone is:

> A user can add tasks in the TUI, see them on mobile PWA, complete/reschedule them on either device, and trust that sync will not lose data.

Until that works, all extra features are secondary.

---

## Final guiding principle

Every implementation decision should serve this goal:

```txt
A fast personal task system that feels native in the terminal, usable on mobile, syncs automatically for free, survives offline use, and does not lose user data.
```

If a feature or architecture choice does not support that goal, do not build it yet.
