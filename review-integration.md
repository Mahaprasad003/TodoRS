# Phase 8 Plan Review: Existing Code Integration Gaps

**Date:** 2026-06-11
**Reviewer:** subagent
**Scope:** Verify plan assumptions against actual codebase at `/home/mp/Projects/TodoRS/`

---

## 1. App::new() Signature — Feasible, No Signature Change Needed

**Actual signature** (`crates/todomrs-tui/src/app.rs:84`):
```rust
pub fn new(
    user_id: Uuid,
    device_id: Uuid,
    task_store: TaskStore,
    op_store: OperationStore,
    project_store: ProjectStore,
    recurrence_store: RecurrenceRuleStore,
) -> Self
```

**Plan assumption:** "Do not change the `App::new()` signature."

**Finding:** The plan correctly assumes the current signature. Adding the three new fields (`sync_client`, `sync_status`, `last_synced_seq`) and initializing them to `None`, `Disabled`, and `0` inside `App::new()` is straightforward. No constructor changes are required.

---

## 2. ui.rs Status Bar — Room Exists But Tight

**Actual layout** (`crates/todomrs-tui/src/ui.rs:38`):
```rust
.constraints([
    Constraint::Min(0),
    Constraint::Length(3),
    Constraint::Length(1),
])
```

The status bar is exactly **1 line high** (`chunks[2]`). The current `draw_status_bar` renders:
- `TodoRS` badge
- View name
- `[P]` filter indicator (conditional)
- `│` separator
- Shortcuts: `q Quit`, `? Help`, `a Add`, `e Edit`, `x Toggle`, `d Del`, `C Clear`, `/ Search`

**Plan:** Add sync indicator symbol + color, and `S Sync` shortcut.

**Finding:** Adding the indicator and shortcut is technically possible, but the bar is already dense. On narrow terminals (< 80 cols), the rightmost shortcuts will be clipped by the `Paragraph` width. The plan does not mention handling overflow or truncation. **This is a layout risk, not a compile blocker.**

**Recommendation:** Consider condensing existing shortcuts (e.g., merge `? Help` with `q Quit` spacing) or omitting the `S Sync` shortcut text if the sync indicator itself is enough visual affordance.

---

## 3. recurrence_store.rs — Method Name Mismatch (`get` vs `get_by_id`)

**Actual API** (`crates/todomrs-store/src/recurrence_store.rs:40`):
```rust
pub async fn get_by_id(&self, id: Uuid) -> Result<Option<RecurrenceRule>>
```

**Plan assumption:** Task 4's `apply_remote_operation()` calls:
```rust
if self.recurrence_store.get(op.entity_id).await?.is_some() {
    return Ok(());
}
```

**Finding:** The store does **not** have a `.get()` method. It has `.get_by_id()`. The plan's Task 8 says "Check if `get(id)` exists. If not, add it." The method exists under a different name, but the plan's core logic in Task 4 uses the wrong name.

**Fix:** Either update all Task 4 calls to use `get_by_id`, or add a `pub async fn get(&self, id: Uuid) -> Result<Option<RecurrenceRule>>` alias that delegates to `get_by_id`. The alias is safer to keep the plan's prose intact.

---

## 4. main.rs — Terminal Setup Order Is Wrong in Plan Example

**Actual flow** (`crates/todomrs-tui/src/main.rs:28-51`):
```rust
// main()
enable_raw_mode()?;                 // 1. Terminal raw mode
execute!(..., EnterAlternateScreen, EnableMouseCapture)?;
let backend = CrosstermBackend::new(stdout);
let mut terminal = Terminal::new(backend)?;
let result = run_async(&mut terminal).await;   // 2. App logic
```

Inside `run_async()`:
```rust
let db = Database::new(...).await?;
// ... stores ...
let mut app = App::new(...);
app.refresh_tasks().await?;
// event loop
```

**Plan assumption:** Task 5 says:
> "In `run_async()`, BEFORE terminal setup: `let config = config::Config::load()?;`"
> "Create sync client and login (before terminal enters raw mode — so errors are visible)."

**Finding:** The plan's comment says "before terminal enters raw mode," but the proposed code is placed **inside `run_async()`**, which is called **after** `enable_raw_mode()` in `main()`. Therefore, config/login errors would print inside the alternate screen buffer, not in normal terminal mode, making them hard to read or invisible.

**Fix:** Move `config::Config::load()`, sync client creation, and login into `main()` **before** the `enable_raw_mode()` call. Pass the resulting `Option<SyncClient>` (or a thin init struct) into `run_async()` as a parameter. Example sketch:

```rust
async fn main() -> Result<()> {
    let config = config::Config::load()?;
    let sync_client = init_sync_client(&config).await;
    // NOW enter raw mode
    enable_raw_mode()?;
    // ...
    let result = run_async(&mut terminal, sync_client).await;
    // ...
}
```

---

## 5. todomrs-tui/Cargo.toml — serde/serde_json Already Present

**Actual file** (`crates/todomrs-tui/Cargo.toml:15-16`):
```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

**Plan assumption:** Config.rs needs serde/serde_json; the plan claims "no new deps needed."

**Finding:** Verified. Both are already declared. `anyhow` is also present. No new dependencies are required for the `config.rs` module.

---

## 6. Serialization Helpers — Reusable Code Exists in `recurrence_store.rs`

**Actual code:**
- `crates/todomrs-sync/src/operations.rs:267` — `fn serialize_enum<T: serde::Serialize>(value: &T) -> String` (private)
- `crates/todomrs-store/src/recurrence_store.rs:178` — `fn deserialize_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T>` (private)

**Plan assumption:** Task 4 adds two helpers in `app.rs`:
```rust
fn deserialize_recurrence_kind(s: &str) -> RecurrenceKind { ... }
fn deserialize_anchor_mode(s: &str) -> AnchorMode { ... }
```

**Finding:** The plan's helpers are manual string-match deserializers. The existing `deserialize_enum` in `recurrence_store.rs` already does the same job generically (wraps the string in JSON quotes and calls `serde_json::from_str`). It is currently private.

**Options:**
1. **Keep plan as-is:** The manual helpers are fine, simple, and avoid making store internals public.
2. **Reuse existing code:** Make `deserialize_enum` in `recurrence_store.rs` a `pub` helper and use it from `app.rs`. This reduces duplication.

**Recommendation:** Option 1 is acceptable for Phase 8. Option 2 is a minor cleanup for later. Neither is a blocker.

---

## 7. "No new deps needed" — Verified

**Actual dependencies in `crates/todomrs-tui/Cargo.toml`:**
- `todomrs-core`, `todomrs-store`, `todomrs-sync` — already present
- `serde`, `serde_json` — already present
- `anyhow`, `chrono`, `uuid`, `tokio` — already present
- `ratatui`, `crossterm` — already present

**Plan assumption:** "No new deps needed for todomrs-tui."

**Finding:** **Confirmed.** The only required crates for the plan (`serde`, `serde_json`, `anyhow`, `chrono`, `uuid`, `tokio`, `todomrs-sync`) are already in the TUI crate's manifest.

---

## Additional Notes

### Note A: `Config::config_path()` visibility
The plan's Task 1 code defines `config_path()` as a private `fn`, but Task 5 says:
> "Make `config_path()` public on Config so the error message can show the path."

The plan is self-aware of this gap. Ensure the final implementation makes `config_path()` `pub`.

### Note B: `SyncClient::is_authenticated()` — does not exist yet
The plan correctly identifies this as a new addition in Task 2. The current `client.rs` only has `access_token: Option<String>` (private field). The plan's Task 4 `sync()` method calls `client.is_authenticated()`, so Task 2 must land before Task 4.

### Note C: `task_store.soft_delete` vs `project_store.soft_delete`
Both exist. The plan uses `task_store.soft_delete()` for Task Delete and generic Delete fallback. This is correct.

### Note D: `task_store.get_by_id`, `project_store.get_by_id`, `op_store.get_unsynced`, `op_store.mark_synced`
All exist and match the plan's signatures. No gaps here.

### Note E: `last_synced_seq` persistence
The plan explicitly states this is in-memory only for Phase 8 and is acceptable because apply is idempotent. This is a reasonable trade-off.

### Note F: `project_store` does not have `create` with `sort_order` defaulting
The plan's Project Create remote op constructs a `Project` with `sort_order: 0`. The actual `Project` struct in `todomrs-core` supports this. No issue.

---

## Summary Table

| # | Item | Plan Assumption | Reality | Impact |
|---|------|-----------------|---------|--------|
| 1 | `App::new()` signature | Stays same | Stays same | ✅ No issue |
| 2 | Status bar space | Room for sync indicator | 1-line, dense | ⚠️ Layout risk on narrow terminals |
| 3 | `recurrence_store.get()` | Exists | Named `get_by_id()` | ❌ Compile error if not fixed |
| 4 | `main.rs` setup order | Config before raw mode | Code shown inside `run_async()` | ❌ Errors would print in raw mode |
| 5 | `serde/serde_json` | Already present | Present | ✅ No issue |
| 6 | Serialization helpers | New ones in `app.rs` | `deserialize_enum` exists in store | 📝 Note: could reuse existing code |
| 7 | No new deps | True | True | ✅ No issue |

---

## Blockers

1. **`recurrence_store.get()` method name:** The plan's Task 4 calls `.get()` but the actual store exposes `.get_by_id()`. Add a `get()` alias or update Task 4 calls.
2. **`main.rs` ordering:** The plan's example code places config loading inside `run_async()`, which happens **after** `enable_raw_mode()`. Refactor to load config and init sync client in `main()` before terminal setup.

## Notes

1. **Status bar overflow:** Consider testing on 80×24 terminal. The additional `S Sync` text and sync glyph may be truncated. Consider dynamic truncation or merging help text.
2. **Reuse `deserialize_enum`:** If you want to keep `app.rs` lean, expose `deserialize_enum` from `todomrs-store` and use it for `RecurrenceKind` and `AnchorMode` deserialization.
3. **Plan self-consistency:** The plan correctly notes that `config_path()` must be public and that `is_authenticated()` must be added first. These are well-sequenced in the "Implementation Order" section.
