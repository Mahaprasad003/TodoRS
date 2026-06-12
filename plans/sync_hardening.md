# Sync Hardening — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Eliminate cross-account sync corruption by partitioning local state by authenticated account, making the authenticated Supabase user the canonical synced identity, and hardening both TUI and PWA against shared-device multi-login bugs.

**Architecture:** The TUI must stop treating the current working directory and a synthetic local user UUID as the storage boundary. Instead, compute an **effective account identity** at startup, open a **per-account local database**, and keep a **single global device ID** per install. In synced mode, the canonical local `user_id` must equal the authenticated Supabase user ID. The PWA must apply the same account-scoping discipline to `sync_state` and unsynced-operation queries.

**Tech Stack:** Rust (`sqlx`, `tokio`, `uuid`, `chrono`), SvelteKit PWA (`idb`, TypeScript), Supabase Edge Functions.

---

## Findings (root cause summary)

### Primary root cause in the TUI

1. **The TUI always opens one shared SQLite file**:
   - `crates/todomrs-tui/src/main.rs:161`
   - Current path: `sqlite://./todomrs.db?mode=rwc`
   - Effect: every authenticated account shares one local state store.

2. **The TUI persists a synthetic local user ID unrelated to Supabase auth**:
   - `crates/todomrs-tui/src/main.rs:170`
   - Current file: `.todomrs_user_id`
   - Effect: local rows are keyed by a synthetic UUID while remote rows are keyed by Supabase auth user ID.

3. **The TUI persists `last_synced_at` in one global metadata row**:
   - `crates/todomrs-tui/src/app.rs:148-162`
   - `crates/todomrs-tui/src/app.rs:1120-1125`
   - Effect: account A can advance the download cursor and cause account B to skip remote operations.

4. **The TUI notification metadata is also global**:
   - `crates/todomrs-tui/src/notifications.rs:21,44,48,100,123-139`
   - Keys: `last_daily_notify`, `notified_tasks`
   - Effect: notification state bleeds across accounts.

5. **The current `remote_max_seq` glue logic is compensating for the wrong storage boundary**:
   - `crates/todomrs-store/src/operation_store.rs:16-18,112-128`
   - `crates/todomrs-tui/src/app.rs:1040-1052`
   - Effect: it mitigates duplicate `(user_id, device_id, seq)` collisions after account switching, but does not fix the real cross-account contamination.

### Secondary hardening gaps in the PWA

1. **PWA `sync_state` is keyed only by `device_id`**:
   - `pwa/src/lib/db/index.ts:40-41`
   - `pwa/src/lib/db/sync-state.ts:4-24`
   - Effect: browser account A and account B can reuse the same download cursor.

2. **PWA uploads all unsynced operations, regardless of current user**:
   - `pwa/src/lib/db/operations.ts:9-12`
   - `pwa/src/lib/sync/client.ts:323-326`
   - Effect: browser multi-login can upload the wrong account’s local operations.

### Verified current behavior from local inspection

- TUI local identity files differ from PWA local identity files.
- Local SQLite contains one synthetic `users` row: `local@todomrs`.
- TUI tests currently pass (`cargo test -q`), which means the bug is architectural/state-boundary related, not a simple failing unit path.

---

## Decision: target architecture

### Canonical rules

1. **Authenticated Supabase user ID is the canonical synced `user_id`.**
   - In synced mode, local entities and local operations must use the remote auth user UUID.
   - The old synthetic `.todomrs_user_id` must not be used for synced accounts.

2. **Device ID remains global per installation, not per account.**
   - Same device, different accounts is fine because backend uniqueness is per user + device + seq.

3. **Each synced account gets its own local TUI database file.**
   - Example path shape: `~/.local/share/todomrs/accounts/<supabase_user_id>/todomrs.db`
   - Offline/no-auth mode gets its own isolated DB, e.g. `~/.local/share/todomrs/accounts/offline-local/todomrs.db`

4. **Do not mutate or “upgrade in place” the legacy `./todomrs.db`.**
   - Leave it untouched.
   - New code stops using it.
   - If desired, warn the user once that old state exists and is now quarantined.

5. **Per-account DB partitioning is the primary fix.**
   - Do not paper over this with more seq hacks.
   - Keep `remote_max_seq` only as a defensive guard for account/device replay safety.

---

## Bugs found and exact fixes

| Bug | Where | Why it breaks | Exact fix |
|---|---|---|---|
| Shared TUI DB across accounts | `crates/todomrs-tui/src/main.rs` | All accounts share tasks, ops, metadata, sync cursor | Open DB from per-account path derived from effective user ID |
| Synthetic local user ID used in synced mode | `crates/todomrs-tui/src/main.rs`, `app.rs` | Local rows keyed by fake user while remote rows keyed by real auth user | In synced mode, set `App.user_id = supabase_user_id` |
| Global `last_synced_at` | `crates/todomrs-tui/src/app.rs` | Account B can inherit account A’s download cursor | Per-account DB solves this automatically; keep helper methods for load/save |
| Global notification metadata | `crates/todomrs-tui/src/notifications.rs` | Reminder suppression leaks across accounts | Per-account DB solves this automatically |
| `remote_max_seq` acting as core logic | `operation_store.rs`, `app.rs` | Seq fix compensates for wrong storage boundary | Keep as defensive logic only; stop relying on it to separate accounts |
| PWA `sync_state` keyed by device only | `pwa/src/lib/db/index.ts`, `sync-state.ts` | Cursor can bleed across browser accounts | Key by `(user_id, device_id)` or a derived `account_key` |
| PWA `getUnsyncedOperations()` unscoped | `pwa/src/lib/db/operations.ts`, `sync/client.ts` | Current user can upload another user’s pending ops | Require `userId` argument and filter before upload |
| Login UX implies wiping config to switch account | `crates/todomrs-tui/src/main.rs` | Encourages manual destructive switching | Make `todomrs login` overwrite credentials cleanly and rely on per-account DB isolation |

---

## Non-goals

- Do **not** redesign the sync protocol.
- Do **not** introduce server-side snapshots, CRDTs, or a new conflict model.
- Do **not** automatically import the legacy shared `./todomrs.db` into per-account DBs.
- Do **not** add a new JS test framework just for this change.

---

## Implementation tasks

## Task 1: Introduce TUI account-scoped storage helpers

**Objective:** Create one place that computes config/data paths, device ID location, offline user fallback, and per-account DB URLs.

**Files:**
- Create: `crates/todomrs-tui/src/storage.rs`
- Modify: `crates/todomrs-tui/src/main.rs`

**Step 1: Create `storage.rs` with pure helpers**

Implement helpers with no DB side effects except directory creation where explicitly needed:

- `fn config_dir() -> PathBuf`
- `fn data_dir() -> PathBuf`
- `fn device_id_path() -> PathBuf`
- `fn offline_user_id_path() -> PathBuf`
- `fn legacy_db_path() -> PathBuf` returning `PathBuf::from("./todomrs.db")`
- `fn database_path_for_user(user_id: &str) -> PathBuf`
- `fn sqlite_url_for_path(path: &Path) -> String`
- `fn load_or_create_uuid(path: &Path) -> Uuid`
- `fn ensure_parent_dir(path: &Path) -> Result<()>`

**Required path rules:**
- Config dir: `~/.config/todomrs`
- Data dir: `~/.local/share/todomrs`
- Global device id: `~/.config/todomrs/device_id`
- Offline fallback user id: `~/.config/todomrs/offline_user_id`
- Per-account DB: `~/.local/share/todomrs/accounts/<account-key>/todomrs.db`
- Offline account key: literal `offline-local`

**Step 2: Add tests inside `storage.rs`**

Add pure unit tests for:
- database path generation
- sqlite URL generation
- no cwd dependence
- account key `offline-local`

**Step 3: Wire `mod storage;` into `main.rs`**

Do not change runtime behavior yet beyond compiling the new module.

**Verification:**
- Run: `cargo test -q`
- Expected: pass

---

## Task 2: Compute effective account identity before opening the TUI database

**Objective:** Make startup choose the correct account-scoped DB and canonical local user ID.

**Files:**
- Modify: `crates/todomrs-tui/src/main.rs`
- Modify: `crates/todomrs-tui/src/config.rs`
- Modify: `crates/todomrs-tui/src/app.rs`

**Step 1: Define startup identity rules**

In `main.rs`, after `init_sync_client(&config).await` returns:

- If sync client exists and `supabase_user_id()` is present:
  - `effective_user_id = supabase_user_id`
  - `account_key = effective_user_id.to_string()`
  - `effective_email = config.email.clone()`
- Else:
  - `effective_user_id = storage::load_or_create_uuid(storage::offline_user_id_path())`
  - `account_key = "offline-local"`
  - `effective_email = "local@todomrs".to_string()`

**Step 2: Replace shared DB open**

Replace:
- `Database::new("sqlite://./todomrs.db?mode=rwc")`

With:
- compute per-account DB path via `storage::database_path_for_user(...)`
- ensure parent dirs exist
- open DB using the generated sqlite URL

**Step 3: Replace cwd identity files**

Remove synced-mode dependence on:
- `.todomrs_user_id`
- `.todomrs_device_id`

Use instead:
- global device id at `storage::device_id_path()`
- offline user id only in no-auth mode

**Step 4: Ensure local `users` row matches effective user**

Replace current synthetic insert with:
- `INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)`
- bind `effective_user_id`
- bind `effective_email`

**Step 5: Initialize `App::new(...)` with canonical user ID**

Pass `effective_user_id` into the `App` constructor.

**Step 6: Update login UX string**

In `cmd_login()`, remove:
- `rm ~/.config/todomrs/config.json && todomrs login`

Replace with language that says login simply overwrites stored credentials; local state is now account-scoped.

**Verification:**
- Run: `cargo test -q`
- Expected: pass
- Manual: log in as account A, inspect DB path exists under `~/.local/share/todomrs/accounts/<A>/todomrs.db`

---

## Task 3: Quarantine legacy shared local state safely

**Objective:** Stop using the old cwd-scoped files without destroying them.

**Files:**
- Modify: `crates/todomrs-tui/src/main.rs`
- Optionally create: `crates/todomrs-tui/src/storage.rs` helper functions if needed

**Step 1: Detect legacy artifacts**

At startup, detect existence of:
- `./todomrs.db`
- `./.todomrs_user_id`
- `./.todomrs_device_id`

**Step 2: Do not migrate automatically**

Do not copy or transform legacy DB content into the new per-account DB.

Reason: legacy DB is potentially mixed across accounts and cannot be mapped safely.

**Step 3: Emit a one-time warning (stderr is fine)**

If legacy artifacts exist, print a concise message:
- old shared local state detected
- new build uses per-account storage
- legacy files are left untouched

Do not block startup.

**Verification:**
- Manual only
- With legacy files present, app starts and uses new DB path

---

## Task 4: Remove cross-account sync-state bleed in the TUI

**Objective:** Make sync state and metadata naturally account-scoped by using the per-account DB.

**Files:**
- Modify: `crates/todomrs-tui/src/app.rs`
- Modify: `crates/todomrs-tui/src/notifications.rs`

**Step 1: Keep `load_sync_state()` but treat it as per-account DB state**

No schema change is required. The fix is that each account now has a different DB.

However, clean up the code:
- extract a small helper in `app.rs` for loading `last_synced_at`
- extract a helper for persisting `last_synced_at`
- keep the metadata key literal `last_synced_at` if desired

**Step 2: Keep notification metadata keys unchanged**

Because DBs are now per-account, the existing keys become safe:
- `last_daily_notify`
- `notified_tasks`

Do not add unnecessary schema changes here.

**Step 3: Add comments explaining why this is now safe**

In both `app.rs` and `notifications.rs`, document:
- metadata is per-account because DB is per-account
- these keys are intentionally local to that DB

**Verification:**
- Run: `cargo test -q`
- Expected: pass

---

## Task 5: Make synced-mode identity canonical and simplify seq/auth glue

**Objective:** Ensure local rows, local ops, and remote ops all agree on the same `user_id` in synced mode.

**Files:**
- Modify: `crates/todomrs-tui/src/app.rs`
- Modify: `crates/todomrs-store/src/operation_store.rs`
- Modify: `crates/todomrs-store/tests/operation_store_test.rs`

**Step 1: Keep `self.user_id` canonical**

After Task 2, `self.user_id` already equals Supabase auth user ID in synced mode. Audit the following call sites and make sure no synthetic-user assumptions remain:

- `Task::new(self.user_id, ...)`
- `Project::new(self.user_id, ...)`
- `self.task_store.get_all(self.user_id)`
- `self.project_store.get_all(self.user_id)`
- `self.op_store.get_unsynced(self.user_id)`
- `self.op_store.get_next_seq(self.user_id, self.device_id)`

These should now all be correct without additional mapping.

**Step 2: Simplify `expected_user_id` logic**

Current code in `app.rs:1099-1102` derives `expected_user_id` from the sync client and falls back to `self.user_id`.

After canonicalization, replace this with a simpler rule:
- `expected_user_id = self.user_id`

**Step 3: Keep `remote_max_seq` as defensive logic only**

Current code in `app.rs:1040-1052` should still work, but query remote max using the canonical `self.user_id`.

Avoid dual local/remote user ID branches.

**Step 4: Add tests for `OperationStore` defensive seq logic**

In `crates/todomrs-store/tests/operation_store_test.rs`, add:

- `test_get_next_seq_uses_remote_max_when_local_empty`
- `test_get_next_seq_prefers_local_when_local_exceeds_remote`
- `test_get_unsynced_is_user_scoped`

These tests should stay pure store-level tests.

**Verification:**
- Run: `cargo test -q`
- Expected: pass

---

## Task 6: Harden duplicate-upload handling without redesigning the server contract

**Objective:** Keep upload idempotency, but stop treating duplicate handling as the primary account-separation mechanism.

**Files:**
- Modify: `crates/todomrs-tui/src/app.rs`
- Optionally modify comments in: `supabase/functions/upload-operations/index.ts`

**Step 1: Leave current server behavior intact unless clearly broken**

Do not redesign the Supabase function in this pass.

Current behavior:
- server inserts operations with authenticated `user.id`
- duplicate key errors are tolerated
- client marks duplicates as synced

This remains acceptable **after** account partitioning is fixed.

**Step 2: Improve comments**

In TUI sync code, update comments to explain:
- duplicate upload tolerance is an idempotency guard
- it is no longer relied upon to mask account-switch state corruption

**Step 3: Do not add speculative upsert logic**

Avoid changing the edge function to `upsert` unless you first verify the exact remote unique constraints. The current schema was not fully inspected from Supabase migrations, and a wrong `onConflict` target can break replay semantics.

**Verification:**
- Run: `cargo test -q`
- Expected: pass

---

## Task 7: Fix PWA sync-state scoping by account

**Objective:** Make browser sync state keyed by both current user and device, not device alone.

**Files:**
- Modify: `pwa/src/lib/db/index.ts`
- Modify: `pwa/src/lib/db/schema.ts`
- Modify: `pwa/src/lib/db/sync-state.ts`
- Modify: `pwa/src/lib/sync/client.ts`

**Step 1: Bump IndexedDB schema version**

In `pwa/src/lib/db/schema.ts`:
- bump `DB_VERSION` from `1` to `2`

**Step 2: Redefine sync-state keying**

Choose one of these and implement consistently:

### Preferred option: derived string key
Add field:
- `account_key: string` where value is `${user_id}:${device_id}`

Then:
- make `sync_state` store keyPath `account_key`
- keep `device_id` and `user_id` as payload fields

This is simpler than array key paths across the rest of the code.

**Step 3: Update sync-state helpers**

Change signatures to require both user and device:
- `getSyncState(userId: string, deviceId: string)`
- `initSyncState(userId: string, deviceId: string)`
- `updateSyncState(state: SyncStateRecord)` where `state.account_key` is always populated

**Step 4: Add IndexedDB upgrade logic**

In `pwa/src/lib/db/index.ts`, handle upgrade from v1 safely:
- create new `sync_state` store shape if needed
- if migration complexity is annoying, delete and recreate `sync_state` store during version bump (acceptable because sync cursor is rebuildable from server)
- do not delete tasks/projects/operations stores

**Step 5: Update all call sites**

In `pwa/src/lib/sync/client.ts` update:
- `initSyncState(...)`
- `getSyncState(...)`
- `updateSyncState(...)`
- `bootstrapAfterAuth(...)`

**Verification:**
- Run: `cd pwa && npm run check`
- Expected: pass
- Manual: login as A then B in same browser profile; each account gets its own `last_synced_at`

---

## Task 8: Fix PWA unsynced-operation scoping

**Objective:** Ensure browser uploads only the current user’s pending operations.

**Files:**
- Modify: `pwa/src/lib/db/operations.ts`
- Modify: `pwa/src/lib/sync/client.ts`

**Step 1: Change the operations helper API**

Replace:
- `getUnsyncedOperations(): Promise<OperationRecord[]>`

With:
- `getUnsyncedOperations(userId: string): Promise<OperationRecord[]>`

Implementation rule:
- fetch by `user_id` index first
- filter `!synced_at`
- sort by `seq`

Do not read all operations across all users.

**Step 2: Update sync client upload path**

In `pwa/src/lib/sync/client.ts`, change:
- `const unsyncedOps = await getUnsyncedOperations();`

To:
- `const unsyncedOps = await getUnsyncedOperations(currentUserId);`

Guard `currentUserId` first as needed.

**Step 3: Optional follow-up hardening**

Also consider scoping `getAllOperations()` if it is used in UI or debugging. Not required if unused in sync.

**Verification:**
- Run: `cd pwa && npm run check`
- Expected: pass

---

## Task 9: Validate remote-op apply path after identity canonicalization

**Objective:** Ensure downloaded remote operations still materialize correctly once `self.user_id` equals the auth user.

**Files:**
- Modify: `crates/todomrs-tui/src/app.rs`

**Step 1: Audit all `apply_remote_operation()` branches**

Current code creates local rows with `self.user_id` in places like:
- task create
- project create
- recurrence-generated task creation
n
This becomes correct once `self.user_id` is canonical.

**Step 2: Check “skip our own device” rule remains valid**

Keep:
- skip if `op.device_id == self.device_id`

This is still right because device ID remains installation-wide.

**Step 3: Avoid hidden fallback to synthetic IDs**

Search for and remove any logic that assumes:
- local fake user for downloaded rows
- auth user only for remote filtering

Use one identity consistently.

**Verification:**
- Run: `cargo test -q`
- Expected: pass

---

## Task 10: End-to-end manual verification matrix

**Objective:** Prove the fix with the exact failure mode that motivated this work.

**Files:**
- No source changes required
- Optional docs note in plan footer after validation is complete

### Scenario A: TUI account isolation
1. Login as account A in TUI.
2. Confirm DB path exists under `accounts/<A>/todomrs.db`.
3. Create one task in TUI.
4. Sync.
5. Switch credentials to account B via `todomrs login`.
6. Launch TUI again.
7. Confirm DB path exists under `accounts/<B>/todomrs.db`.
8. Confirm account A task does **not** appear locally before sync.
9. Sync account B.
10. Confirm only account B data appears.

### Scenario B: TUI ↔ PWA same account
1. Login TUI as account A.
2. Create task in TUI, sync.
3. Open PWA as account A.
4. Confirm task appears.
5. Create task in PWA.
6. Sync PWA.
7. Reopen/sync TUI.
8. Confirm task appears in TUI.

### Scenario C: PWA multi-login hardening
1. In one browser profile, login account A.
2. Create local unsynced task.
3. Sign out and login account B.
4. Confirm sync state for B starts from B’s own cursor, not A’s.
5. Confirm B upload path does not upload A’s unsynced ops.

### Scenario D: Legacy artifact quarantine
1. Leave old `./todomrs.db` in repo root.
2. Start new TUI build.
3. Confirm a warning is shown.
4. Confirm new per-account DB is used instead.
5. Confirm old cwd DB remains untouched.

**Verification commands:**
- Rust: `cargo test -q`
- PWA: `cd pwa && npm run check`

---

## Search checklist for the implementer

Before opening a PR, run these repo-wide searches and confirm each result is intentional.

### TUI searches
- search for `./todomrs.db`
- search for `.todomrs_user_id`
- search for `.todomrs_device_id`
- search for `local@todomrs`
- search for `supabase_user_id()` fallback logic
- search for `last_synced_at` direct metadata SQL

### PWA searches
- search for `getSyncState(`
- search for `initSyncState(`
- search for `updateSyncState(`
- search for `getUnsyncedOperations(`
- search for `device_id`-only sync state access

Expected result: no remaining synced-mode dependence on cwd identity files or device-only PWA sync state.

---

## Risks and mitigations

### Risk 1: User loses visibility into legacy local-only data
**Mitigation:** Do not delete legacy files. Emit a warning. Keep new code isolated from old DB.

### Risk 2: Offline mode breaks
**Mitigation:** Keep a dedicated offline fallback user ID and offline-local DB path.

### Risk 3: PWA IndexedDB upgrade accidentally wipes too much
**Mitigation:** Only recreate `sync_state` store if needed. Do not touch entity or operation stores.

### Risk 4: Seq logic regresses for empty local DB on existing device
**Mitigation:** Keep and test `remote_max_seq` defensive behavior.

---

## Definition of done

This work is done only when all of the following are true:

- TUI no longer opens `./todomrs.db`
- TUI no longer uses `.todomrs_user_id` in synced mode
- TUI uses authenticated Supabase user ID as canonical local synced `user_id`
- TUI metadata and notification state are isolated by per-account DB
- PWA `sync_state` is keyed by account + device, not device alone
- PWA upload path fetches unsynced operations for the current user only
- `cargo test -q` passes
- `cd pwa && npm run check` passes
- Manual multi-login scenarios pass

---

## Recommended commit breakdown

1. `refactor(tui): add account-scoped storage helpers`
2. `refactor(tui): open per-account databases and canonicalize synced user identity`
3. `docs(tui): warn about legacy shared local state`
4. `test(store): cover remote max seq fallback and user scoping`
5. `refactor(pwa): scope sync state by account and device`
6. `fix(pwa): scope unsynced operation uploads by current user`
7. `docs(sync): clarify duplicate upload handling and account isolation`

---

## Final note for the execution agent

Do **not** start by editing `seq` math again. The correct first move is to fix the storage boundary and identity model. Once local state is partitioned by authenticated account, the observed sync failures become straightforward and the existing seq safeguard becomes a small defensive layer instead of the thing holding the system together.
