# Phase 3 Handoff

> Context for starting Phase 3 in a fresh session.
> Read this alongside `plans/phase_03.md`.

## Current state of the codebase

### Workspace (4 crates)
- **todomrs-core** — domain types, parser (to be built here)
- **todomrs-store** — CRUD for tasks/projects/tags via sqlx
- **todomrs-sync** — empty scaffold
- **todomrs-tui** — empty scaffold with `println`

### Key deviations from the original Phase 2 plan that affect Phase 3

| Topic | What the phase_03.md plan assumes | What actually exists |
|-------|-----------------------------------|---------------------|
| `Task::delete()` | Assumes a `delete()` method | Domain has `delete()` (sets `deleted_at`), store has `hard_delete()` (removes row) and `soft_delete()` (sets `deleted_at`) |
| `tag_ids` column | Plan doesn't heavily use it | Removed from schema; `task_tags` junction table instead. Domain `Task.tag_ids: Vec<Uuid>` still exists as a Rust-level field |
| Enum serde | Plan writes raw string values | Enums use `#[serde(rename_all = "snake_case")]` — `"pending"`, `"completed"`, `"none"`, `"high"`, `"daily"`, etc. |
| `serde_json` serialization | Plan may assume `to_string()` | Store uses `serialize_enum()` helper that unwraps serde_json quoting |

### Current API surface (for reference if Phase 3 touches stores)

**TaskStore:**
```rust
create(&Task) -> Result<()>           // transactional (task + tags)
get_by_id(Uuid) -> Result<Option<Task>>
get_all(Uuid) -> Result<Vec<Task>>    // excludes soft-deleted
update(&Task) -> Result<()>           // transactional (task + tags)
hard_delete(Uuid) -> Result<()>       // removes row
soft_delete(Uuid) -> Result<()>       // sets deleted_at
```

**ProjectStore / TagStore** follow the same pattern with `hard_delete`/`soft_delete` where applicable.

### Where to build Phase 3

- **Parser**: `crates/todomrs-core/src/parser.rs` — new module, register in `lib.rs`
- **Recurrence engine**: `crates/todomrs-core/src/recurrence.rs` — new module
- Tests go inline in each module `#[cfg(test)] mod tests { ... }`

### What the plan gets right

The Phase 3 plan's `ParsedTask` struct, `NaturalLanguageParser` API, recurrence rule calculation, and test patterns are all compatible with the current code. No structural changes needed.

### Quick start

```bash
cd ~/Projects/TodoRS
cargo build           # should succeed
cargo test            # 26 tests, all passing
# Then follow phase_03.md Task 1 → 2 → 3
```
