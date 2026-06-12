<div align="center">
  <br/>
  <img src="pwa/static/icon-192.svg" width="96" height="96" alt="TodoRS"/>
  <br/>
  <h1>TodoRS</h1>
  <p>
    <strong>A task manager that lives in your terminal.</strong>
  </p>
  <p>
    <em>(...and also on your phone. but mostly the terminal.)</em>
  </p>
  <p>
    <code>Terminal-first · Mobile PWA · Offline-capable · No subscriptions</code>
  </p>
  <p>
    <br/>
    <a href="#-why-another-todo-app">Why?</a> ·
    <a href="#-features">Features</a> ·
    <a href="#-quick-start">Quick Start</a> ·
    <a href="#-natural-language-quick-add">NLP</a> ·
    <a href="#-architecture-decisions">Architecture</a>
  </p>
  <br/>
</div>

---

> *"The most over-engineered way to feel productive."*

<br/>

## 🗿 Why another todo app?

There was no good reason to reinvent the wheel. I just wanted to see if I could build something usable.

It's yet another todo app that sucks, but it's *mine*. And I like it. And mostly I just wanted something in the terminal, lol.

| | TodoRS | Todoist | Things 3 | Superproductivity |
|---|---|---|---|---|
| **Terminal UI** | ✅ Built-in Rust TUI | ✅ CLI (`td`) | ❌ | ❌ |
| **Mobile** | ✅ PWA (Android, iOS) | ✅ Native apps | ✅ iOS, iPad, Watch | ✅ |
| **Desktop app** | ✅ TUI + PWA | ✅ Windows, macOS, Linux | ✅ macOS only | ✅ Windows, macOS, Linux |
| **Offline-first** | ✅ SQLite / IndexedDB | ✅ | ✅ | ✅ |
| **Natural language input** | ✅ `p1 +proj @tag every week` | ✅ | ✅ Basic | ❌ |
| **Recurring tasks** | ✅ Daily to yearly, custom intervals | ✅ | ✅ | ✅ |
| **Time tracking** | ❌ | ✅ In-app timer | ❌ | ✅ Native |
| **Jira / GitHub integration** | ❌ | ✅ Via API | ❌ | ✅ Jira, GitHub, GitLab |
| **Desktop notifications** | ✅ notify-rust | ✅ | ❌ | ✅ |
| **Open source** | ✅ MIT | ✅ CLI only | ❌ | ✅ |
| **Monthly cost** | **$0** | $0–$5/mo | ~$50 one-time | $0 |
| **Self-hosted backend** | ✅ Supabase (free) | ❌ | ❌ | ✅ Bring your own |
| **Built by one person** | ✅ Yes | ❌ Company | ❌ Company | ✅ Community |

Side note: Todoist actually has a pretty solid [CLI tool](https://github.com/Doist/todoist-cli) now — kudos to them. Superproductivity's Jira integration is genuinely impressive if you need that. Things 3's design is gorgeous and it won an Apple Design Award for good reason.

But none of them let me press `L` in my terminal, authenticate, and start adding tasks with `buy milk tomorrow 3pm p1 +errands` — all without leaving the keyboard, all without a subscription, all synced to a PWA on my phone. So I built the one that does.

<br/>

---

<br/>

## 🖥️ Where does it run?

| Platform | TUI | PWA |
|---|---|---|
| **Linux** | ✅ Full support (notifications, everything) | ✅ Any browser |
| **macOS** | ✅ Compiles and runs (notifications work via macOS NSNotifications) | ✅ Any browser |
| **Windows** | ✅ Compiles and runs (notifications work via winrt-notification) | ✅ Any browser |
| **Android** | ❌ | ✅ Add to home screen as PWA |
| **iOS** | ❌ | ✅ Add to home screen as PWA |

> I develop on Linux. macOS and Windows *should* work — the code compiles, the dependencies are cross-platform, and I've made an effort to keep it portable. But I haven't tested it on those platforms. If something breaks, PRs welcome. The PWA notifications work everywhere the browser supports the Notification API.

## 🎯 Core philosophy

<table>
<tr>
<td width="50%">

### 🏃 The capture loop is sacred

Everything in TodoRS is designed around one loop: **think → type → done**.

The faster you can capture a thought, the less likely you are to lose it. This is why the NLP QuickAdd exists — it turns `buy milk tomorrow 3pm p1 +errands` into a fully structured task in one keystroke.

No modals. No dropdowns. No clicking through date pickers.

</td>
<td width="50%">

### 🔄 The interface is the database

There's no "sync" button you need to remember to press.

The TUI and the PWA talk to the same data store (Supabase), and they sync automatically every 30 seconds. Add a task on your phone, it appears on your laptop. Complete it on your laptop, it's done on your phone.

It's not magic — it's just a well-designed operation log.

</td>
</tr>
<tr>
<td width="50%">

### 🎛️ Features over configuration

**Zero settings screens. Zero toggles. Zero preference panels.**

If a behaviour is worth having, it should be the default. If it's not worth being the default, it's probably not worth having at all.

Notifications fire exactly three times — morning brief, task becomes due, task becomes overdue — each exactly once, never again. No "notification preferences" needed.

</td>
<td width="50%">

### 🗑️ Delete before you add

Every feature went through one filter: *would a normal person use this every day?*

❌ Kanban boards<br/>
❌ Gantt charts<br/>
❌ Habit streaks (not yet, anyway)<br/>
❌ Jira/GitHub/Slack integrations<br/>
❌ Collaboration, teams, workspaces<br/>
❌ Subscription plans<br/>

The app knows what it is — a personal task manager — and it stays in its lane.

</td>
</tr>
</table>

<br/>

---

<br/>

## ✨ Features

<div align="center">

| Feature | TUI | PWA |
|---|---|---|
| **Add tasks via natural language** | ✅ `buy milk tomorrow 3pm p1` | ✅ Same NLP, ported to TS |
| **Quick Add** (bare text) | ✅ Press `a` | ✅ QuickAdd bar |
| **Full create modal** (project, date, priority, repeat) | ✅ inline | ✅ Bottom sheet with pickers |
| **Edit task** | ✅ Inline text edit | ✅ Tap → pre-filled modal |
| **Complete / uncomplete** | ✅ Press `x` | ✅ Checkbox or swipe right ➡️ |
| **Delete** | ✅ Press `d` | ✅ Trash icon or swipe left ⬅️ |
| **Search** | ✅ Press `/` | ✅ Press `/` or tap 🔍 |
| **Views: Inbox, Today, Upcoming, Projects** | ✅ | ✅ |
| **Completed archive** | ✅ View::Completed | ✅ `/completed` route |
| **Priority** (none/low/medium/high/urgent) | ✅ | ✅ Color-coded left border 🟥🟧🟦 |
| **Recurring tasks** (daily/weekly/monthly/yearly) | ✅ | ✅ |
| **Due dates with time** | ✅ | ✅ Date + time picker |
| **Projects with colors** | ✅ | ✅ |
| **NLP project creation** (`+project`) | ✅ | ✅ Auto-creates on first use |
| **Sync across devices** | ✅ Every 30s | ✅ Every 30s + visibility change |
| **Offline support** | ✅ SQLite | ✅ IndexedDB |
| **Notifications** | ✅ Desktop (`notify-rust`) | ✅ Browser Notification API |
| **Morning brief** | ✅ *"Today: buy milk, pay bills"* | ✅ Same |
| **Task due / overdue alerts** | ✅ Once per task | ✅ Once per task |
| **Dark theme** | ✅ TokyoNight | ✅ Linear-inspired |
| **Keyboard shortcuts** | ✅ Full navigation | ✅ `/` for search |
| **Swipe gestures** | ❌ | ✅ Right=complete, left=delete |
| **Multi-device sync** | ✅ | ✅ |
| **Personal** (single user) | ✅ No sign-up | ✅ Sign-in only |

</div>

<br/>

---

<br/>

## ⚡ How it works

```
┌─────────────────────┐       ┌──────────────────┐       ┌──────────────────┐
│                     │       │                  │       │                  │
│   ┌───────────┐     │       │                  │       │     ┌──────── ┐  │
│   │  Rust TUI │     │       │    Supabase      │       │     │SvelteKit│  │
│   │  (ratatui)│◄────┼───────┤                  ├───────┼────►│  PWA    │  │
│   │           │     │       │  Postgres + Auth │       │     │         │  │
│   │  SQLite   │     │       │  Edge Functions  │       │     │IndexedDB│  │
│   │           │     │       │                  │       │     │         │  │
│   │notify-rust│     │       │                  │       │     │  SW 🛰️  │  │
│   └───────────┘     │       │                  │       │     └──────── ┘  │
│                     │       │                  │       │                  │
└─────────────────────┘       └──────────────────┘       └──────────────────┘
```

Every mutation — create task, update title, toggle complete, delete — generates an **operation**. Operations are stored locally (SQLite on the TUI, IndexedDB on the PWA) and synced to Supabase asynchronously.

When the other device syncs, it downloads new operations, skips its own (identified by device ID), and **replays** the rest. The result is a simple, conflict-free merge that works even when both devices edit the same task.

> **No CRDTs. No vector clocks. Just an append-only log with per-device sequence numbers.**
>
> It's boring. It works.

<br/>

---

<br/>

## 🚀 Quick start

### Prerequisites

| Tool | Version | Why |
|---|---|---|
| 🦀 **Rust** | ≥ 1.75 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| 📦 **Node.js** | ≥ 18 | [nodejs.org](https://nodejs.org) |
| 🗃️ **SQLite** | ≥ 3.x | Usually pre-installed. Check: `sqlite3 --version` |
| ☁️ **Supabase account** | Free tier | [supabase.com](https://supabase.com) |

<br/>

### 1. 📥 Clone the repo

```bash
git clone https://github.com/YOUR_USERNAME/todomrs.git
cd todomrs
```

### 2. 🗄️ Set up the TUI database

```bash
# Install sqlx-cli (Rust SQL toolkit)
cargo install sqlx-cli --no-default-features --features sqlite

# Create the SQLite database
export DATABASE_URL="sqlite://./todomrs.db"
sqlx database create
sqlx migrate run
```

### 3. ☁️ Set up Supabase

<table>
<tr>
<td>

**Step 1 — Create a project**

Go to [supabase.com](https://supabase.com) and hit **New project**. Pick any name and region. The free tier gives you 500MB database — more than enough for years of tasks.

</td>
<td>

**Step 2 — Initialize the schema**

Once your project is ready, open the **SQL Editor** and paste the contents of `backend/migrations/001_init.sql`. Hit ▶️ run. This creates the `operations`, `sync_state`, and `devices` tables.

</td>
</tr>
<tr>
<td>

**Step 3 — Create your user**

Go to **Authentication → Users → Add User**. Enter your email and password. This is *your* account — nobody else can sign up (the PWA doesn't have registration).

</td>
<td>

**Step 4 — Get your credentials**

Go to **Project Settings → API**. Copy:
- **Project URL** (looks like `https://xxxxx.supabase.co`)
- **anon public key** (long base64 string starting with `eyJ`)

Keep these handy.

</td>
</tr>
</table>

### 4. 🖥️ Configure and run the TUI

```bash
# Run once to generate the config file
cargo run --bin todomrs
```

This creates `~/.config/todomrs/config.json` with placeholder values. Open that file and replace the placeholders:

```json
{
  "supabase_url": "https://YOUR_PROJECT.supabase.co",
  "supabase_api_key": "YOUR_SUPABASE_ANON_KEY",
  "email": "you@email.com",
  "password": "your-password"
}
```

Now run the TUI again:

```bash
cargo run --bin todomrs
```

You should see `Sync login successful` printed before the TUI starts. Hit `?` to see available keybindings.

### 5. 🔄 Change credentials later (no config file editing)

```bash
cargo run --bin todomrs login

Email: new@email.com
Password: 
Login successful. Credentials saved to config.
```

No config file editing needed. The CLI handles everything.

### 6. 📱 Set up the PWA

```bash
cd pwa
npm install
```

Create a `.env` file in `pwa/`:

```
VITE_SUPABASE_URL=https://YOUR_PROJECT.supabase.co
VITE_SUPABASE_ANON_KEY=your-anon-key
```

Start the dev server:

```bash
npm run dev
```

Open the URL shown (typically `http://localhost:5173`). Sign in with the same credentials you created in Supabase.

### 7. 🌐 Deploy to the world

```bash
cd pwa
npx vercel --prod
```

**Before deploying**, make sure to set these environment variables in your Vercel dashboard (Settings → Environment Variables):

| Name | Value |
|---|---|
| `VITE_SUPABASE_URL` | `https://xxxxx.supabase.co` |
| `VITE_SUPABASE_ANON_KEY` | `eyJ...` |

Now open `https://todors-pwa.vercel.app` on your phone. Tap **Share → Add to Home Screen**. You now have a native-feeling PWA that syncs with your terminal.

<br/>

---

<br/>

## 🧠 Natural language quick-add

This is the heart of TodoRS. Type what you think. The computer figures it out.

```bash
buy milk tomorrow 3pm p1 +errands @groceries
water plants every 2 days
submit report due:friday p2
call dentist on monday 10am
read chapter 5 every! week             # completion-anchored
meditate wait! every day               # wait before spawning next
```

### Full syntax reference

| Token | Example | Behaviour |
|---|---|---|
| `+project` | `+errands` | Assigns to project. **Auto-creates** if it doesn't exist. |
| `@tag` | `@groceries` | Adds a tag. Multiple tags allowed: `@home @urgent` |
| `p1` | `p1` | 🔴 Urgent |
| `p2` | `p2` | 🟠 High |
| `p3` | `p3` | 🔵 Medium |
| `p4` | `p4` | 🟢 Low |
| `today` | — | Due today |
| `tomorrow` | — | Due tomorrow |
| `monday`–`sunday` | `friday` | Due next occurrence of that day |
| `due:date` | `due:friday` | Same as above, but inline |
| `8pm` / `9am` | `3pm` | Sets a specific time |
| `14:30` | — | 24-hour format |
| `every day` | — | Recur daily |
| `every week` | — | Recur weekly |
| `every 3 days` | — | Custom interval |
| `every month` | — | Recur monthly |
| `every year` | — | Recur yearly |
| `every!` | `every! week` | Completion-anchored (next instance spawns when you check it off) |
| `wait!` | `wait! every day` | Don't spawn next instance until current one is completed |

<br/>

---

<br/>

## 🏗️ Architecture decisions

<details>
<summary><strong>🦀 Why Rust for the TUI?</strong></summary>

Rust gives you the performance of C with memory safety. For a terminal app that runs continuously, this means:
- **No garbage collector pauses** — the UI stays responsive even during sync
- **Minimal memory footprint** — ~15MB resident
- **Instant startup** — < 100ms from keystroke to interactive
- **No runtime dependencies** — a single static binary

The alternatives were Go (ugly terminal libs) and Python (too slow, GIL issues). Rust + ratatui is the sweet spot.
</details>

<details>
<summary><strong>⚡ Why SvelteKit for the PWA?</strong></summary>

Svelte compiles away the framework. The result is small bundles (the entire PWA is ~200KB gzipped), fast interactivity, and a codebase that feels like writing vanilla JS with reactivity bolted on.

For a mobile PWA where bundle size and performance matter, this is the right trade-off. React would have been 3x the bundle size for the same functionality.
</details>

<details>
<summary><strong>☁️ Why Supabase?</strong></summary>

Supabase is Postgres with a nice API on top. It gives us:
- **Free tier that lasts forever** — 500MB database, 50K users, no time limit
- **Built-in auth** — email/password, session management, JWT tokens
- **Edge Functions** — Deno-based serverless functions for the sync endpoints
- **Row Level Security** — not that we use it much, but it's there

The alternative was a DIY Go server + SQLite. Supabase is more expensive at scale (not relevant here) but vastly simpler to set up. Zero server maintenance.
</details>

<details>
<summary><strong>📝 Why operation-based sync instead of CRDT?</strong></summary>

CRDTs (Conflict-free Replicated Data Types) are the academically pure approach to multi-device sync. They're also complex, error-prone, and overkill for a single-user app.

TodoRS uses an **append-only operation log**. Every mutation creates an operation. Operations are ordered by per-device sequence numbers and global timestamps. Sync downloads new operations, skips self-generated ones, and replays the rest. Conflicts are resolved by last-writer-wins (by timestamp).

It's not academically pure. It works perfectly for one person with two devices.
</details>

<details>
<summary><strong>🔕 Why no Web Push?</strong></summary>

Honestly, it was too much work. Web Push requires a VAPID key pair, a server-side endpoint to store subscriptions, and a cron job to trigger pushes. For a personal app where the PWA is open on your phone most of the time, the browser `Notification` API (which fires on sync) achieves the same result with **zero infrastructure**.

No Edge Functions. No subscription storage. No VAPID keys. Just 80 lines of code.
</details>

<br/>

---

<br/>

## 🤖 Built with AI

This project was built in collaboration with several AI models, each used for what they're best at:

| Model | Role | Why |
|---|---|---|
| **Qwen 3.7 Plus** (`qwen3.7-plus`) | Planner | Strong at breaking down vague requirements into structured, actionable plans. Used for architecture design and implementation planning. |
| **DeepSeek V4 Flash** (`deepseek-v4-flash`) | Worker, Scout | Fast, efficient, good at executing well-defined tasks. Used for the bulk of implementation and codebase exploration. |
| **Kimi K2.6** (`kimi-k2.6`) | Reviewer, Oracle | Strong critical thinking and edge-case detection. Used for code review, architecture review, and catching bugs before they ship. |
| **Kimi K2.5** (`kimi-k2.5`) | Researcher | Good at synthesizing web research into concise summaries. Used for researching competitor features and API behaviors. |
| **GPT-5.5** | Hard debugging | When everything else failed, this was the last resort. Used for the trickiest root-cause analysis (sync bugs, timezone issues, serialization problems). |

Each model was assigned a specific role based on its strengths — planning, execution, review, research, or deep debugging. The orchestrator (this session) coordinated them, made final decisions, and handled the glue code.

---

## 🗺️ Project structure

```
todomrs/
├── Cargo.toml                          # Rust workspace
├── migrations/                         # SQLite migrations (TUI)
├── backend/migrations/                 # Supabase SQL migrations
├── supabase/functions/
│   ├── upload-operations/index.ts      # Edge Function: upload ops
│   └── get-operations/index.ts         # Edge Function: download ops
│
├── crates/
│   ├── todomrs-core/                   # Domain models, NLP parser, recurrence engine
│   ├── todomrs-store/                  # SQLite store (tasks, projects, ops, reminders)
│   ├── todomrs-sync/                   # Sync client + operation log
│   └── todomrs-tui/                    # TUI app (ratatui + crossterm)
│       ├── src/app.rs                  # Event loop, state, keybindings
│       ├── src/ui.rs                   # Rendering
│       ├── src/main.rs                 # Entry point + CLI login command
│       └── src/notifications.rs        # TUI notification logic
│
├── pwa/                                # SvelteKit PWA
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/             # Svelte components
│   │   │   ├── stores/                 # Tasks, projects, auth, search, edit
│   │   │   ├── db/                     # IndexedDB (schema + CRUD)
│   │   │   ├── sync/client.ts          # PWA sync engine
│   │   │   ├── parser.ts               # NLP parser (TypeScript)
│   │   │   └── notifications.ts        # Browser notification logic
│   │   └── routes/                     # Pages (inbox, today, upcoming, projects, completed)
│   └── static/                         # Icons, manifest, robots
│
└── README.md                           # 📄 You are here
```

<br/>

---

<br/>

<div align="center">
  <br/>
  <p>
    <strong>Built with</strong>
    <br/>
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"/>
    <img src="https://img.shields.io/badge/SvelteKit-FF3E00?style=for-the-badge&logo=svelte&logoColor=white"/>
    <img src="https://img.shields.io/badge/Supabase-3ECF8E?style=for-the-badge&logo=supabase&logoColor=white"/>
    <img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white"/>
  </p>
  <br/>
  <p>
    <sub>MIT License · Built by <a href="https://github.com/Mahaprasad003">@Mahaprasad003</a> · <a href="https://todors-pwa.vercel.app">todors-pwa.vercel.app</a></sub>
  </p>
  <br/>
</div>
