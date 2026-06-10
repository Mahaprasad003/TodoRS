# Phase 7: Backend API — Supabase Setup

## Session Goal

Set up the sync backend using Supabase (free tier). Create the database schema, authentication, and API endpoints for operation upload/download. By the end of this session, you should have a working backend that can receive and serve operations.

## Expected Outcome

- Supabase project created and configured
- Database schema with operations table
- Authentication setup (email/password or magic link)
- API endpoints for:
  - POST /operations — upload operations
  - GET /operations?since=SEQ — get operations since sequence
  - GET /snapshot — get latest snapshot
  - POST /snapshot — upload snapshot
- Basic authentication working
- API can be tested with curl/httpie
- Backend ready for TUI sync client

## Context

Phase 6 is complete. You have:
- Fully functional TUI with task CRUD
- Natural language input working
- Operation log system in place
- All changes persisted to local SQLite

Now you'll build the backend that enables sync between devices. We'll use Supabase because it provides Postgres, auth, and REST API on a generous free tier.

## Prerequisites

- Supabase account (free tier)
- Supabase CLI installed (optional but helpful)
- All previous phases complete

## Tasks

### Task 1: Create Supabase Project and Database Schema

**Objective:** Set up Supabase project and create the operations table.

**Steps:**

1. Go to https://supabase.com and sign up/login

2. Create a new project:
   - Name: `todomrs-sync`
   - Database password: (save this securely)
   - Region: Choose closest to you
   - Wait for project to initialize

3. Get your project credentials:
   - Go to Project Settings → API
   - Copy `Project URL` and `anon public` key
   - Save these — you'll need them

4. Create database schema via SQL Editor:

```sql
-- Create operations table
CREATE TABLE operations (
    op_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    device_id UUID NOT NULL,
    seq BIGINT NOT NULL,
    entity TEXT NOT NULL,
    entity_id UUID NOT NULL,
    op_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    synced_at TIMESTAMPTZ,
    UNIQUE(user_id, device_id, seq)
);

-- Create index for efficient querying
CREATE INDEX idx_operations_user_device_seq ON operations(user_id, device_id, seq);
CREATE INDEX idx_operations_synced_at ON operations(synced_at);
CREATE INDEX idx_operations_user_created ON operations(user_id, created_at);

-- Create sync_state table
CREATE TABLE sync_state (
    user_id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    last_local_seq BIGINT NOT NULL DEFAULT 0,
    last_synced_seq BIGINT NOT NULL DEFAULT 0,
    last_sync_at TIMESTAMPTZ
);

-- Create snapshots table
CREATE TABLE snapshots (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    device_id UUID NOT NULL,
    snapshot_seq BIGINT NOT NULL,
    state_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_snapshots_user_device ON snapshots(user_id, device_id, snapshot_seq);

-- Create devices table for tracking
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ
);

CREATE INDEX idx_devices_user ON devices(user_id);
```

5. Execute the SQL in Supabase SQL Editor

6. Verify tables created:
```sql
SELECT table_name FROM information_schema.tables WHERE table_schema = 'public';
```

Expected: operations, sync_state, snapshots, devices tables exist.

**Commit:** Save SQL to `backend/migrations/001_init.sql` in your repo.

```bash
mkdir -p backend/migrations
# Save SQL to file
git add backend/
git commit -m "feat: add backend database schema for sync"
```

---

### Task 2: Set Up Authentication

**Objective:** Configure Supabase auth for user registration and login.

**Steps:**

1. In Supabase Dashboard, go to Authentication → Providers

2. Enable Email provider (should be enabled by default)

3. Configure email settings:
   - Confirm email: Disable for now (development)
   - Or enable and use a test email

4. Create a test user via SQL Editor:

```sql
-- Insert test user (replace with your email)
INSERT INTO auth.users (
    instance_id,
    id,
    email,
    encrypted_password,
    email_confirmed_at,
    raw_app_meta_data,
    raw_user_meta_data,
    created_at,
    updated_at,
    role
)
VALUES (
    '00000000-0000-0000-0000-000000000000',
    gen_random_uuid(),
    'test@example.com',
    crypt('password123', gen_salt('bf')),
    NOW(),
    '{"provider": "email", "providers": ["email"]}',
    '{}',
    NOW(),
    NOW(),
    'authenticated'
);
```

5. Or create user via Supabase Auth API:

```bash
curl -X POST "https://YOUR_PROJECT.supabase.co/auth/v1/signup" \
  -H "apikey: YOUR_ANON_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "password123"
  }'
```

6. Test login:

```bash
curl -X POST "https://YOUR_PROJECT.supabase.co/auth/v1/token?grant_type=password" \
  -H "apikey: YOUR_ANON_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "password123"
  }'
```

Expected: Returns access_token and refresh_token.

**Commit:**
```bash
git add .
git commit -m "feat: configure Supabase authentication"
```

---

### Task 3: Create API Endpoints with Supabase Edge Functions

**Objective:** Create serverless functions for operation upload/download.

**Steps:**

1. Install Supabase CLI:
```bash
npm install -g supabase
```

2. Initialize Supabase in your project:
```bash
cd ~/Projects/TodoRS
supabase init
```

3. Create edge function for operations upload:

```bash
supabase functions new upload-operations
```

4. Edit `supabase/functions/upload-operations/index.ts`:

```typescript
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

Deno.serve(async (req) => {
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    const authHeader = req.headers.get('Authorization')
    if (!authHeader) {
      throw new Error('Missing authorization header')
    }

    const supabase = createClient(
      Deno.env.get('SUPABASE_URL') ?? '',
      Deno.env.get('SUPABASE_ANON_KEY') ?? '',
      { global: { headers: { Authorization: authHeader } } }
    )

    const { data: { user } } = await supabase.auth.getUser()
    if (!user) {
      throw new Error('Not authenticated')
    }

    const { operations } = await req.json()

    // Insert operations
    const { error } = await supabase
      .from('operations')
      .insert(operations.map((op: any) => ({
        ...op,
        user_id: user.id,
      })))

    if (error) throw error

    return new Response(
      JSON.stringify({ success: true }),
      { headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  } catch (error) {
    return new Response(
      JSON.stringify({ error: error.message }),
      { status: 400, headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  }
})
```

5. Create edge function for operations download:

```bash
supabase functions new get-operations
```

6. Edit `supabase/functions/get-operations/index.ts`:

```typescript
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

Deno.serve(async (req) => {
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    const authHeader = req.headers.get('Authorization')
    if (!authHeader) {
      throw new Error('Missing authorization header')
    }

    const supabase = createClient(
      Deno.env.get('SUPABASE_URL') ?? '',
      Deno.env.get('SUPABASE_ANON_KEY') ?? '',
      { global: { headers: { Authorization: authHeader } } }
    )

    const { data: { user } } = await supabase.auth.getUser()
    if (!user) {
      throw new Error('Not authenticated')
    }

    const url = new URL(req.url)
    const sinceSeq = url.searchParams.get('since') || '0'

    const { data, error } = await supabase
      .from('operations')
      .select('*')
      .eq('user_id', user.id)
      .gt('seq', parseInt(sinceSeq))
      .order('seq', { ascending: true })

    if (error) throw error

    return new Response(
      JSON.stringify({ operations: data }),
      { headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  } catch (error) {
    return new Response(
      JSON.stringify({ error: error.message }),
      { status: 400, headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  }
})
```

7. Deploy functions:
```bash
supabase functions deploy upload-operations
supabase functions deploy get-operations
```

8. Test upload endpoint:

```bash
curl -X POST "https://YOUR_PROJECT.supabase.co/functions/v1/upload-operations" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [{
      "op_id": "00000000-0000-0000-0000-000000000001",
      "device_id": "00000000-0000-0000-0000-000000000002",
      "seq": 1,
      "entity": "Task",
      "entity_id": "00000000-0000-0000-0000-000000000003",
      "op_type": "Create",
      "payload": {"title": "Test task"},
      "created_at": "2026-06-10T12:00:00Z"
    }]
  }'
```

Expected: Returns `{ "success": true }`

**Commit:**
```bash
git add supabase/
git commit -m "feat: create Supabase edge functions for operation sync"
```

---

### Task 4: Create Rust Client Library for Backend

**Objective:** Create a Rust library that wraps the Supabase API calls.

**Steps:**

1. Add dependencies to `crates/todomrs-sync/Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
```

2. Create `crates/todomrs-sync/src/client.rs`:

```rust
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct SyncClient {
    client: Client,
    base_url: String,
    api_key: String,
    access_token: Option<String>,
}

impl SyncClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            access_token: None,
        }
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/auth/v1/token?grant_type=password", self.base_url))
            .header("apikey", &self.api_key)
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        self.access_token = data.get("access_token").and_then(|v| v.as_str()).map(String::from);

        Ok(self.access_token.clone().unwrap_or_default())
    }

    pub async fn upload_operations(&self, operations: Vec<super::operations::Operation>) -> Result<()> {
        let token = self.access_token.as_ref().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        self.client
            .post(format!("{}/functions/v1/upload-operations", self.base_url))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "operations": operations }))
            .send()
            .await?;

        Ok(())
    }

    pub async fn get_operations(&self, since_seq: i64) -> Result<Vec<super::operations::Operation>> {
        let token = self.access_token.as_ref().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let response = self
            .client
            .get(format!("{}/functions/v1/get-operations?since={}", self.base_url, since_seq))
            .header("apikey", &self.api_key)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let operations = data
            .get("operations")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Deserialize operations
        let ops: Vec<super::operations::Operation> = operations
            .into_iter()
            .filter_map(|op| serde_json::from_value(op).ok())
            .collect();

        Ok(ops)
    }
}
```

3. Update `crates/todomrs-sync/src/lib.rs`:

```rust
pub mod operations;
pub mod snapshot;
pub mod client;

pub use operations::*;
pub use snapshot::Snapshot;
pub use client::SyncClient;
```

4. Verify it compiles:
```bash
cargo build
```

Expected: Compiles successfully.

**Commit:**
```bash
git add .
git commit -m "feat: create Rust sync client library for Supabase"
```

---

## Verification

Test the full flow:

1. Login and get access token
2. Upload a test operation
3. Download operations
4. Verify operation appears in Supabase database

```bash
# Login
curl -X POST "https://YOUR_PROJECT.supabase.co/auth/v1/token?grant_type=password" \
  -H "apikey: YOUR_ANON_KEY" \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123"}'

# Upload operation (use access_token from above)
curl -X POST "https://YOUR_PROJECT.supabase.co/functions/v1/upload-operations" \
  -H "Authorization: Bearer ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"operations": [...]}'

# Get operations
curl "https://YOUR_PROJECT.supabase.co/functions/v1/get-operations?since=0" \
  -H "Authorization: Bearer ACCESS_TOKEN"
```

Expected: Operations upload and download successfully.

## Pitfalls

1. **Don't commit secrets.** Never commit Supabase URL, API keys, or passwords to git.

2. **Don't skip CORS headers.** Edge functions need CORS for browser clients.

3. **Don't ignore authentication.** All endpoints must verify user identity.

4. **Don't forget to deploy.** Edge functions must be deployed to work.

## Handoff to Next Phase

Phase 8 will assume:
- Supabase backend working
- Edge functions deployed
- Rust client library ready
- Authentication configured

Phase 8 will integrate the sync client into the TUI.
