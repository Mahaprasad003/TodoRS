# Recurrence in TodoRS

TodoRS supports recurring tasks inspired by [SuperProductivity](https://github.com/super-productivity/super-productivity)'s model. Recurrence rules are stored separately and each occurrence is an independent task instance. When you complete a recurring task, a new instance is spawned with the next due date.

---

## Quick Start

Create a recurring task from the add prompt (`a`):

```
a Water plants every day
a Weekly review every week
a Pay rent every month
a Backup server every 2 weeks
a Meditate every! day
a Laundry wait! every week
```

---

## Syntax Reference

### Basic patterns

| Input | Rule created |
|-------|-------------|
| `every day` | Daily, interval 1 |
| `every week` | Weekly, interval 1 |
| `every month` | Monthly, interval 1 |
| `every year` | Yearly, interval 1 |
| `every 2 days` | Daily, interval 2 |
| `every 3 weeks` | Weekly, interval 3 |
| `every 6 months` | Monthly, interval 6 |

### Prefix modifiers

Two optional prefixes modify recurrence behavior. They go **before** `every`:

```
wait! every day
every! week
wait! every! 2 weeks
```

---

## `every` vs `every!` — Anchor Mode

Controls **what date the next occurrence is calculated from**.

### `every` (default) — Schedule-anchored

The next due date is calculated from the **original scheduled date** of the current instance. The schedule stays fixed — completing late doesn't shift the pattern.

```
Task: "Laundry every 7 days"
Created June 1st (due June 1st)
                                    ▲── anchor
You complete it on:                 June 3rd (2 days late)
Next instance due:                  June 8th  (June 1st + 7 days)
                                              ▲ NOT June 10th
```

**Good for:** Fixed-schedule tasks like meetings, standing appointments, rent.

**Examples:**

```
Standup every day              → always due today, advance from today
Pay rent every month           → always due on the Nth, advance from original date
Quarterly review every 3 months → Q1 → Q2 → Q3 regardless of when you check it off
```

### `every!` — Completion-anchored

The next due date is calculated from **when you actually completed** the current instance. The schedule shifts based on real completion time.

```
Task: "Review PR every! 7 days"
Created June 1st (due June 1st)
                                    ▲── anchor
You complete it on:                 June 3rd (2 days late)
Next instance due:                  June 10th (June 3rd + 7 days)
                                              ▲ NOT June 8th
```

**Good for:** Interval-based tasks where the gap between completions matters more than the calendar date.

**Examples:**

```
Water plants every! 2 days       → 2 days after you last watered
Change sheets every! 2 weeks     → 2 weeks after you last changed them
Take vitamin every! day          → 24 hours after you last took one
Backup server every! 3 days      → 3 days after the last backup
```

### Quick comparison table

| You want this | Use | Why |
|---|---|---|
| "Standup at 9am every day" | `every day` | Fixed schedule — doesn't matter when you check it off |
| "Water plants — 2 days after I actually watered" | `every! 2 days` | Interval matters, calendar date doesn't |
| "Pay rent on the 1st" | `every month` | Fixed date |
| "Change sheets when I last did it" | `every! 2 weeks` | Gap matters more than day-of-month |
| "Weekly team standup" | `every week` | Same day each week |
| "Medicate 8 hours after last dose" | `every! day` | (8h intervals with custom time) |

---

## `wait!` — Wait for Completion Mode

**Note:** The `wait!` prefix is parsed and stored in the database, but the behavioral logic (suppressing next instance until current is done) is not yet implemented. The flag is stored for forward compatibility.

### How it will work (future)

By default (no `wait!`), completing a recurring task immediately spawns the next instance. If you fall behind, instances pile up:

```
Without wait! (current behavior):
  Mon: Create "Weekly review" ──── complete it ──→ spawns Tue
  Tue: New instance appears ────── skip it
  Wed: Skip again
  Thu: Complete Tue's ────────────────────────────→ spawns Wed
  Now you have: 2 completed + 1 pending
```

With `wait!`, the next instance only spawns after the current one is completed. If you skip a day, the next one waits:

```
With wait! (future behavior):
  Mon: Create "Weekly review wait! every week" ──→ spawns Tue
  Tue: Complete it ───────────────────────────────→ spawns Wed
  Wed: Skip (don't complete)
  Thu: Still on Wed's instance ───────────────────→ no Thu spawned yet
  When you complete Wed: ─────────────────────────→ spawns Thu
```

This prevents pile-up for sequential tasks where each step depends on the previous one being done.

### Combined with anchor modes

```
wait! every day          → schedule-anchored + wait
wait! every! day         → completion-anchored + wait
wait! every! 2 weeks     → 2 weeks from completion + wait
```

---

## How It Works Internally

### Data model

```
recurrence_rules table
┌──────────┬──────────┬────────┬──────────┬──────────┬──────┐
│ id (PK)  │ task_id  │ kind   │ interval │ wait_for │ anchor │
│          │ (FK)     │        │          │ _compl.  │ _mode  │
├──────────┼──────────┼────────┼──────────┼──────────┼────────┤
│ uuid-1   │ uuid-a   │ daily  │ 1        │ false    │ sched  │
│ uuid-2   │ uuid-b   │ weekly │ 2        │ true     │ compl  │
└──────────┴──────────┴────────┴──────────┴──────────┴────────┘

tasks table
┌──────────┬─────────────────────┬────────────────────┐
│ id (PK)  │ title               │ recurrence_rule_id │
│          │                     │ (→ recurrence.id)  │
├──────────┼─────────────────────┼────────────────────┤
│ uuid-a   │ Water plants        │ uuid-1             │ ← original task
│ uuid-c   │ Water plants        │ uuid-1             │ ← spawned instance (same rule!)
│ uuid-d   │ Water plants        │ uuid-1             │ ← another spawned instance
└──────────┴─────────────────────┴────────────────────┘
```

Key points:

- One `RecurrenceRule` ≈ many task instances (one-to-many)
- All instances link back to the same rule via `recurrence_rule_id`
- The rule's `task_id` points to the original task that created it
- In the UI, the ♻ icon + interval indicator (e.g. `♻2w`) is shown for tasks that have a `recurrence_rule_id`

### What happens when you complete a recurring task

```
1. You press x on "Water plants" (due June 1st)
2. System checks: does this task have a recurrence_rule_id?
   └─ Yes → looks up the rule by rule.id (not task.id)
3. Anchor mode check:
   ├─ Schedule → anchor = task.due_at (or created_at if no due date)
   └─ Completion → anchor = task.completed_at
4. RecurrenceEngine computes next_due = anchor + interval
5. New task created with:
   └─ title, project, tags, priority, due_at = next_due copied from completed task
   └─ recurrence_rule_id = same rule.id
6. New task persisted + sync operation recorded
7. Status bar shows: "Completed: Water plants → Next: every day"
```

### Edit round-trip

When you press `e` on a recurring task, the edit buffer shows the full natural-language string including prefixes:

| Rule stored | Edit buffer shows |
|---|---|
| `every day` | `Water plants every day` |
| `every! day` | `Water plants every! day` |
| `wait! every week` | `Weekly review wait! every week` |
| `wait! every! day` | `Meditate wait! every! day` |
| `every 2 weeks` | `Review PR every 2 weeks` |
| `every! 3 days` | `Backup every! 3 days` |

Editing the buffer and pressing Enter re-parses the whole thing. If you change `every day` to `every 3 days`, the rule is updated. If you remove `every day` entirely, the rule is deleted from the database.

---

## UI Indicators

In the task list, recurring tasks show a compact indicator next to the title:

```
□ Water plants [12/06]           ← normal task
□ ♻ Water plants [12/06]         ← recurring daily
□ ♻2w Review PR [25/06]          ← every 2 weeks
□ ♻3m Pay rent [01/09]           ← every 3 months
□ !! ♻2w Review PR [25/06]       ← every 2 weeks, high priority
```

The indicator format is `♻` followed by an optional interval number:

| Rule | Indicator |
|------|-----------|
| `every day` | `♻` |
| `every week` | `♻` |
| `every month` | `♻` |
| `every 2 days` | `♻2d` |
| `every 3 weeks` | `♻3w` |
| `every 6 months` | `♻6m` |
| `every 2 years` | `♻2y` |

---

## Changing or Stopping Recurrence

### Via edit (`e`)

1. Press `e` on a recurring task
2. Edit the buffer to change the recurrence pattern
3. Press Enter to save

| Edit action | Result |
|---|---|
| `every day` → `every 3 days` | Rule updated to interval=3 |
| `every day` → `every week` | Rule kind changed to Weekly |
| `every day` → just the title | Rule deleted from DB, task becomes one-shot |
| Add `every month` to a one-shot task | New rule created |
| `every! day` → `every day` | Anchor mode changed to Schedule |

### Via delete (`d`)

Deleting a recurring task (soft-delete) removes that specific instance. The recurrence rule and other instances are unaffected.

---

## Current Known Limitations

- **`wait!` behavior not yet enforced** — the flag is stored but doesn't suppress spawning yet
- **No "stop recurring" keybinding** — must edit the task to remove the recurrence pattern
- **No skip/reschedule** — can't advance a recurring task to the next occurrence without completing
- **No completion history** — completed instances show in the Completed view but there's no "completed 5 times" counter
- **No advanced patterns** — `every Mon, Wed, Fri` or `every 15th` not yet supported
- **No projected future instances** — only the current/next occurrence is shown

---

## Summary Cheatsheet

```
Examples:

  Basic recurrence:
    every day
    every week
    every month
    every year
    every 3 days
    every 2 weeks

  Completion-anchored (next due from completion date):
    every! day
    every! 2 weeks
    every! 3 months

  Wait for completion (future — stored but not enforced):
    wait! every day
    wait! every week

  Combined:
    wait! every! day
    wait! every! 2 weeks
```
