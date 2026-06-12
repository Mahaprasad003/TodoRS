# Final Changes: PWA → TUI Sync Root Cause and Fix Plan

## Scope

This document covers only the current known issue:

> Updates originating from the PWA are persisted to Supabase but do not propagate back to the TUI.

No new features, refactors, or architectural changes are proposed here. The goal is a minimal root-cause fix plus verification.

---

## Root cause

The TUI sync client builds the `get-operations` URL with incomplete manual URL encoding.

Relevant file:

```text
crates/todomrs-sync/src/client.rs
```

Current code:

```rust
let since_str = since_time.to_rfc3339();

let response = self
    .client
    .get(format!(
        "{}/functions/v1/get-operations?since={}",
        self.base_url,
        urlencoding(&since_str)
    ))
    .header("apikey", &self.api_key)
    .header("Authorization", format!("Bearer {}", token))
    .send()
    .await?;
```

Current helper:

```rust
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A")
}
```

This encodes `:` but does **not** encode `+`.

A TUI timestamp like this:

```text
1970-01-01T00:00:00+00:00
```

is sent as:

```text
1970-01-01T00%3A00%3A00+00%3A00
```

In URL query parameters, `+` is interpreted as a space. Supabase receives:

```text
1970-01-01T00:00:00 00:00
```

Postgres rejects that as an invalid `timestamptz`, so the Edge Function returns HTTP 400 and the TUI never downloads/applies remote PWA operations.

---

## Evidence gathered

### 1. PWA mutations are persisted to Supabase

Direct Supabase Edge Function fetch showed:

```text
remote_ops_count: 207
```

Operation counts included PWA-origin lowercase operations:

```text
('task', 'create'): 56
('task', 'update'): 76
('task', 'delete'): 57
('project', 'create'): 7
('project', 'delete'): 7
```

Example PWA-origin create operation in Supabase:

```json
{
  "op_id": "4a3c2405-c8a9-425a-b27d-7065185d7fdf",
  "user_id": "90101d5b-3f62-43dd-a63a-5fa768cfe5be",
  "device_id": "dd8c6784-8d2e-4b3e-b520-77fc3041bde7",
  "seq": 3,
  "entity": "task",
  "entity_id": "0c93c2ac-5d27-4e39-aba7-f981fd698f08",
  "op_type": "create",
  "payload": {
    "task_create": {
      "title": "pwa-test-2",
      "due_at": null,
      "status": "pending",
      "tag_ids": [],
      "priority": "none",
      "project_id": null,
      "description": "",
      "scheduled_at": null,
      "recurrence_rule_id": null
    }
  },
  "created_at": "2026-06-11T14:36:55.116+00:00",
  "synced_at": null
}
```

### 2. Proper encoding makes Supabase return the expected operations

Manual query test:

```text
proper count 207
```

using:

```text
1970-01-01T00%3A00%3A00%2B00%3A00
```

### 3. TUI-style encoding fails

Manual query test with current TUI-style encoding:

```text
rustish HTTP 400
{"error":"invalid input syntax for type timestamp with time zone: \"1970-01-01T00:00:00 00:00\""}
```

Current encoder output:

```text
1970-01-01T00%3A00%3A00+00%3A00
```

Correct output should be:

```text
1970-01-01T00%3A00%3A00%2B00%3A00
```

### 4. Local TUI database confirms remote PWA changes were not applied

Examples:

Remote PWA delete operation exists:

```text
entity_id: 35ca8313-9f3d-4bfe-8505-c51e8f0ced02
op_type: delete
created_at: 2026-06-11T15:15:34.149+00:00
```

But local TUI SQLite row remains undeleted:

```text
title: test
status: pending
deleted_at: null
```

Remote PWA create operation exists:

```text
entity_id: 0c93c2ac-5d27-4e39-aba7-f981fd698f08
title: pwa-test-2
```

But local TUI SQLite row is absent:

```text
local_row: null
```

---

## Exact change to make

### File

```text
crates/todomrs-sync/src/client.rs
```

### Replace manual query construction

Replace this block:

```rust
let response = self
    .client
    .get(format!(
        "{}/functions/v1/get-operations?since={}",
        self.base_url,
        urlencoding(&since_str)
    ))
    .header("apikey", &self.api_key)
    .header("Authorization", format!("Bearer {}", token))
    .send()
    .await?;
```

with this:

```rust
let response = self
    .client
    .get(format!("{}/functions/v1/get-operations", self.base_url))
    .query(&[("since", since_str.as_str())])
    .header("apikey", &self.api_key)
    .header("Authorization", format!("Bearer {}", token))
    .send()
    .await?;
```

Then delete this helper entirely:

```rust
/// Minimal URL-encoding for the timestamp parameter.
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A")
}
```

Reason: `reqwest::RequestBuilder::query` performs correct query parameter encoding, including encoding `+` as `%2B`.

---

## Strongly recommended diagnostic hardening

This is not a feature. It prevents future sync failures from being silently interpreted as “no remote operations”.

### File

```text
crates/todomrs-sync/src/client.rs
```

Current code parses JSON without checking HTTP status:

```rust
let data: serde_json::Value = response.json().await?;
let operations = data
    .get("operations")
    .and_then(|v| v.as_array())
    .cloned()
    .unwrap_or_default();
```

Replace with status-aware handling:

```rust
let status = response.status();
let body = response.text().await?;

if !status.is_success() {
    return Err(anyhow::anyhow!(
        "get-operations failed {}: {}",
        status,
        body
    ));
}

let data: serde_json::Value = serde_json::from_str(&body)?;
let operations = data
    .get("operations")
    .and_then(|v| v.as_array())
    .cloned()
    .unwrap_or_default();
```

Optional but useful: stop silently dropping operation deserialization failures.

Current code:

```rust
let ops: Vec<super::operations::Operation> = operations
    .into_iter()
    .filter_map(|op| serde_json::from_value(op).ok())
    .collect();
```

Better diagnostic version:

```rust
let mut ops = Vec::new();
for op in operations {
    match serde_json::from_value(op.clone()) {
        Ok(parsed) => ops.push(parsed),
        Err(e) => eprintln!("Failed to deserialize remote operation: {}; raw={}", e, op),
    }
}
```

This helps expose legacy/casing problems like old remote operations using `Task` / `Create` instead of `task` / `create`.

---

## What to test

### 1. Unit-level / client-level test for URL encoding

Add or run a focused test that ensures the Rust client can fetch operations using a timestamp containing `+00:00`.

Minimum manual check:

```bash
cargo test -p todomrs-sync
```

Better targeted test shape:

- construct a `since_time` with UTC offset serialized by `to_rfc3339()`
- verify the request uses reqwest query encoding, not manual string concatenation
- if using HTTP mocking, assert `+` becomes `%2B` in the query string

The key regression condition:

```text
+00:00 must not arrive at the server as space + 00:00
```

---

### 2. Integration test: TUI downloads PWA-created task

Manual test steps:

1. Start with TUI configured for Supabase and logged in.
2. In the PWA, create a new task with a unique title, for example:

   ```text
   pwa-to-tui-regression-test
   ```

3. Confirm the PWA sync indicator reaches synced state, or wait for its background sync.
4. In the TUI, trigger manual sync with:

   ```text
   s
   ```

   or wait for the 30-second periodic sync.

5. Expected TUI result:

   - status message includes remote ops, e.g.

     ```text
     Synced (1 remote ops)
     ```

   - the new PWA-created task appears in the TUI list.

6. Optional SQLite verification:

   ```bash
   sqlite3 todomrs.db "select title, deleted_at from tasks where title = 'pwa-to-tui-regression-test';"
   ```

   Expected:

   ```text
   pwa-to-tui-regression-test|null
   ```

---

### 3. Integration test: PWA update propagates to TUI

Manual test steps:

1. Create or choose a task visible in both PWA and TUI.
2. In the PWA, complete/uncomplete the task.
3. Wait for PWA sync.
4. Trigger TUI sync with `s` or wait for periodic sync.
5. Expected:

   - task status changes in TUI
   - `completed_at` is set when completed
   - `completed_at` is cleared or status becomes pending when uncompleted, depending on intended current semantics

SQLite verification:

```bash
sqlite3 todomrs.db "select title, status, completed_at from tasks where title = '<task title>';"
```

---

### 4. Integration test: PWA delete propagates to TUI

Manual test steps:

1. Create a task in TUI or PWA and confirm it exists in both.
2. Delete it from the PWA.
3. Wait for PWA sync.
4. Trigger TUI sync.
5. Expected:

   - task disappears from active TUI views
   - local SQLite row has non-null `deleted_at`

SQLite verification:

```bash
sqlite3 todomrs.db "select title, deleted_at from tasks where title = '<task title>';"
```

Expected:

```text
<task title>|<non-null timestamp>
```

---

### 5. Existing test suite

Run the relevant Rust tests:

```bash
cargo test -p todomrs-sync
cargo test -p todomrs-store
cargo test -p todomrs-tui
```

If time is limited, minimum:

```bash
cargo test -p todomrs-sync
cargo build -p todomrs-tui
```

Previously observed baseline before the fix:

```text
cargo test -p todomrs-sync
4 passed; 0 failed
```

---

## Expected result after the fix

After replacing manual URL construction with `.query(...)`:

1. TUI `get_operations(last_synced_at)` should receive remote PWA operations from Supabase.
2. `App::sync()` should enter the remote apply loop:

   ```rust
   for op in &remote_ops {
       ...
       self.apply_remote_operation(op).await
   }
   ```

3. PWA-created tasks should appear in the TUI.
4. PWA updates should mutate existing TUI tasks.
5. PWA deletes should soft-delete TUI tasks.
6. Sync status should show applied remote operations instead of silently reporting no changes or download failure.

---

## Files involved

### PWA mutation path

```text
pwa/src/lib/stores/tasks.ts
pwa/src/lib/sync/client.ts
pwa/src/lib/db/operations.ts
```

### Supabase backend

```text
supabase/functions/upload-operations/index.ts
supabase/functions/get-operations/index.ts
backend/migrations/001_init.sql
```

### TUI sync path

```text
crates/todomrs-sync/src/client.rs
crates/todomrs-tui/src/app.rs
crates/todomrs-tui/src/main.rs
crates/todomrs-store/src/operation_store.rs
```

---

## Non-root-cause risks found

These are not required for the immediate fix, but they should be known.

### TUI `last_synced_at` is in-memory only

`sync_state` exists, but the TUI currently initializes:

```rust
last_synced_at: chrono::DateTime::from_timestamp(0, 0).unwrap_or(chrono::Utc::now())
```

This means the TUI redownloads from epoch every run. Idempotency mitigates duplicate application, but it is inefficient and increases exposure to old malformed/legacy operations.

Do not fix this as part of the current minimal bug fix unless explicitly planned separately.

### Legacy remote operations may use old enum casing

Remote data included:

```text
('Task', 'Create'): 2
```

Current Rust serde expects snake_case/lowercase values. These legacy operations may be skipped during deserialization. This is separate from the current PWA issue, whose live PWA ops are lowercase and valid.

### Silent deserialization drops

Current code uses:

```rust
.filter_map(|op| serde_json::from_value(op).ok())
```

That makes malformed operations invisible during debugging. Add logging if touching this code.

---

## Minimal commit summary

Suggested commit message:

```text
fix: correctly encode get-operations since query
```

Suggested implementation scope:

1. Use `reqwest::RequestBuilder::query` in `SyncClient::get_operations`.
2. Remove the custom `urlencoding` helper.
3. Add HTTP status-aware error handling for `get_operations`.
4. Run sync client tests and TUI build.
5. Manually verify PWA create/update/delete propagation to TUI.
