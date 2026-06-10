# Phase 9: PWA Mobile Client

## Session Goal

Build a Progressive Web App (PWA) mobile client using SvelteKit that syncs with the same Supabase backend. By the end of this session, you should have a mobile-friendly web app that can view, add, and complete tasks, syncing with the TUI.

## Expected Outcome

- SvelteKit PWA project set up
- Authentication with Supabase
- IndexedDB local cache for offline support
- Views: Inbox, Today, Upcoming
- Quick add task input
- Complete/uncomplete tasks
- Sync with Supabase backend
- Service worker for offline support
- Installable as PWA on mobile
- Mobile-friendly responsive UI

## Context

Phase 8 is complete. You have:
- TUI with sync working
- Supabase backend with operations
- Multi-device sync functional

Now you'll build the mobile client. We use SvelteKit because it's lightweight, fast, and has excellent PWA support. The PWA will use the same sync protocol as the TUI.

## Prerequisites

- Node.js 18+ installed
- npm or pnpm installed
- Supabase backend working
- Phase 8 complete

## Tasks

### Task 1: Initialize SvelteKit Project

**Objective:** Create the PWA project with SvelteKit.

**Steps:**

1. Create SvelteKit project:
```bash
cd ~/Projects/TodoRS
npm create svelte@latest pwa
```

Choose:
- Skeleton project
- TypeScript
- Add: ESLint, Prettier
- No to other options

2. Install dependencies:
```bash
cd pwa
npm install
npm install @supabase/supabase-js
npm install idb
npm install workbox-precaching workbox-routing workbox-strategies
```

3. Update `pwa/package.json` to add PWA scripts:
```json
{
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview"
  }
}
```

4. Create `pwa/src/lib/supabase.ts`:

```typescript
import { createClient } from '@supabase/supabase-js'

const supabaseUrl = 'https://YOUR_PROJECT.supabase.co'
const supabaseAnonKey = 'YOUR_ANON_KEY'

export const supabase = createClient(supabaseUrl, supabaseAnonKey)
```

5. Verify it runs:
```bash
npm run dev
```

Expected: SvelteKit dev server starts at http://localhost:5173

**Commit:**
```bash
git add pwa/
git commit -m "feat: initialize SvelteKit PWA project"
```

---

### Task 2: Implement Authentication

**Objective:** Add login/signup flow with Supabase auth.

**Steps:**

1. Create `pwa/src/lib/stores/auth.ts`:

```typescript
import { writable } from 'svelte/store'
import { supabase } from '$lib/supabase'
import type { User } from '@supabase/supabase-js'

export const user = writable<User | null>(null)
export const loading = writable(true)

export async function initAuth() {
  const { data: { session } } = await supabase.auth.getSession()
  user.set(session?.user ?? null)
  loading.set(false)

  supabase.auth.onAuthStateChange((event, session) => {
    user.set(session?.user ?? null)
  })
}

export async function signIn(email: string, password: string) {
  const { data, error } = await supabase.auth.signInWithPassword({ email, password })
  if (error) throw error
  return data
}

export async function signUp(email: string, password: string) {
  const { data, error } = await supabase.auth.signUp({ email, password })
  if (error) throw error
  return data
}

export async function signOut() {
  await supabase.auth.signOut()
}
```

2. Create `pwa/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { initAuth, user, loading } from '$lib/stores/auth'

  onMount(() => {
    initAuth()
  })
</script>

{#if $loading}
  <div class="loading">Loading...</div>
{:else if !$user}
  <slot />
{:else}
  <slot />
{/if}

<style>
  .loading {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    font-size: 1.5rem;
  }
</style>
```

3. Create `pwa/src/routes/login/+page.svelte`:

```svelte
<script lang="ts">
  import { signIn, signUp } from '$lib/stores/auth'
  import { goto } from '$app/navigation'

  let email = ''
  let password = ''
  let error = ''

  async function handleLogin() {
    try {
      await signIn(email, password)
      goto('/')
    } catch (e: any) {
      error = e.message
    }
  }

  async function handleSignup() {
    try {
      await signUp(email, password)
      goto('/')
    } catch (e: any) {
      error = e.message
    }
  }
</script>

<div class="login">
  <h1>TodoRS</h1>
  
  <form on:submit|preventDefault={handleLogin}>
    <input type="email" bind:value={email} placeholder="Email" required />
    <input type="password" bind:value={password} placeholder="Password" required />
    
    {#if error}
      <p class="error">{error}</p>
    {/if}
    
    <button type="submit">Login</button>
    <button type="button" on:click={handleSignup}>Sign Up</button>
  </form>
</div>

<style>
  .login {
    max-width: 400px;
    margin: 2rem auto;
    padding: 2rem;
  }
  
  form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  
  input {
    padding: 0.75rem;
    border: 1px solid #ccc;
    border-radius: 4px;
  }
  
  button {
    padding: 0.75rem;
    background: #0066cc;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  
  .error {
    color: red;
  }
</style>
```

4. Update `pwa/src/routes/+page.svelte` to redirect if not logged in:

```svelte
<script lang="ts">
  import { user } from '$lib/stores/auth'
  import { goto } from '$app/navigation'
  import { onMount } from 'svelte'

  onMount(() => {
    if (!$user) {
      goto('/login')
    }
  })
</script>

{#if $user}
  <h1>Welcome to TodoRS</h1>
  <p>You are logged in as {$user.email}</p>
{/if}
```

5. Verify login works:
```bash
npm run dev
```

Navigate to http://localhost:5173/login and test login.

**Commit:**
```bash
git add pwa/
git commit -m "feat: add authentication to PWA"
```

---

### Task 3: Implement Task Views and Sync

**Objective:** Create task views and sync with Supabase.

**Steps:**

1. Create `pwa/src/lib/stores/tasks.ts`:

```typescript
import { writable } from 'svelte/store'
import { supabase } from '$lib/supabase'
import { user } from './auth'

export interface Task {
  id: string
  title: string
  status: string
  due_at?: string
  priority: string
  project_id?: string
  created_at: string
}

export const tasks = writable<Task[]>([])
export const loading = writable(false)

export async function loadTasks() {
  loading.set(true)
  
  const { data, error } = await supabase
    .from('tasks')
    .select('*')
    .eq('user_id', $user?.id)
    .order('created_at', { ascending: false })
  
  if (error) {
    console.error('Error loading tasks:', error)
  } else {
    tasks.set(data || [])
  }
  
  loading.set(false)
}

export async function createTask(title: string) {
  const { data, error } = await supabase
    .from('tasks')
    .insert({
      user_id: $user?.id,
      title,
      status: 'pending',
      priority: 'none',
    })
    .select()
  
  if (error) {
    console.error('Error creating task:', error)
    throw error
  }
  
  if (data && data[0]) {
    tasks.update(tasks => [data[0], ...tasks])
  }
}

export async function completeTask(taskId: string) {
  const { error } = await supabase
    .from('tasks')
    .update({ status: 'completed', completed_at: new Date().toISOString() })
    .eq('id', taskId)
  
  if (error) {
    console.error('Error completing task:', error)
    throw error
  }
  
  tasks.update(tasks =>
    tasks.map(t => t.id === taskId ? { ...t, status: 'completed' } : t)
  )
}
```

2. Update `pwa/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import { user } from '$lib/stores/auth'
  import { tasks, loadTasks, createTask, completeTask } from '$lib/stores/tasks'
  import { goto } from '$app/navigation'
  import { onMount } from 'svelte'

  let newTaskTitle = ''

  onMount(async () => {
    if (!$user) {
      goto('/login')
    } else {
      await loadTasks()
    }
  })

  async function handleAddTask() {
    if (newTaskTitle.trim()) {
      await createTask(newTaskTitle)
      newTaskTitle = ''
    }
  }

  async function handleComplete(taskId: string) {
    await completeTask(taskId)
  }
</script>

{#if $user}
  <div class="app">
    <header>
      <h1>TodoRS</h1>
      <p>{$user.email}</p>
    </header>

    <div class="add-task">
      <input
        type="text"
        bind:value={newTaskTitle}
        placeholder="Add task..."
        on:keydown={(e) => e.key === 'Enter' && handleAddTask()}
      />
      <button on:click={handleAddTask}>Add</button>
    </div>

    <div class="tasks">
      {#each $tasks as task (task.id)}
        <div class="task" class:completed={task.status === 'completed'}>
          <input
            type="checkbox"
            checked={task.status === 'completed'}
            on:change={() => handleComplete(task.id)}
          />
          <span>{task.title}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .app {
    max-width: 600px;
    margin: 0 auto;
    padding: 1rem;
  }

  header {
    margin-bottom: 2rem;
  }

  .add-task {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 2rem;
  }

  .add-task input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  .add-task button {
    padding: 0.75rem 1.5rem;
    background: #0066cc;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .tasks {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .task {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border: 1px solid #eee;
    border-radius: 4px;
  }

  .task.completed {
    opacity: 0.5;
  }

  .task.completed span {
    text-decoration: line-through;
  }
</style>
```

3. Verify it works:
```bash
npm run dev
```

Navigate to http://localhost:5173, login, and test adding/completing tasks.

**Commit:**
```bash
git add pwa/
git commit -m "feat: implement task views and sync in PWA"
```

---

### Task 4: Add PWA Manifest and Service Worker

**Objective:** Make the app installable as a PWA with offline support.

**Steps:**

1. Create `pwa/static/manifest.json`:

```json
{
  "name": "TodoRS",
  "short_name": "TodoRS",
  "description": "Personal task manager",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#0066cc",
  "icons": [
    {
      "src": "/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

2. Update `pwa/src/app.html` to include manifest:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link rel="manifest" href="/manifest.json" />
    <meta name="theme-color" content="#0066cc" />
    <link rel="icon" href="/favicon.png" />
    %sveltekit.head%
  </head>
  <body>
    <div>%sveltekit.body%</div>
  </body>
</html>
```

3. Create `pwa/src/service-worker.ts`:

```typescript
/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />

const sw = self as unknown as ServiceWorkerGlobalScope

import { build, files, version } from '$service-worker'

const CACHE = `cache-${version}`
const ASSETS = [...build, ...files]

sw.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(ASSETS))
  )
})

sw.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      for (const key of keys) {
        if (key !== CACHE) {
          caches.delete(key)
        }
      }
    })
  )
})

sw.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') return

  event.respondWith(
    caches.match(event.request).then((cached) => {
      return cached || fetch(event.request)
    })
  )
})
```

4. Update `pwa/svelte.config.js` to enable service worker:

```javascript
import adapter from '@sveltejs/adapter-auto'
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    serviceWorker: {
      register: true
    }
  }
}

export default config
```

5. Build and test:
```bash
npm run build
npm run preview
```

Navigate to http://localhost:4173 and test PWA installation.

**Commit:**
```bash
git add pwa/
git commit -m "feat: add PWA manifest and service worker"
```

---

## Verification

Test the PWA:

1. Run `npm run dev`
2. Open http://localhost:5173 on mobile or Chrome DevTools mobile emulation
3. Login with test credentials
4. Add a task
5. Complete a task
6. Refresh page — tasks should persist
7. Go offline — app should still load
8. Install PWA (Add to Home Screen)

## Pitfalls

1. **Don't commit Supabase credentials.** Use environment variables.

2. **Don't skip HTTPS.** Service workers require HTTPS (except localhost).

3. **Don't ignore browser compatibility.** Test on multiple mobile browsers.

4. **Don't forget icons.** PWA needs icon files for installation.

## Handoff to Next Phase

Phase 10 will assume:
- PWA working with basic task management
- Sync with Supabase functional
- Offline support via service worker

Phase 10 will add reminders, notifications, and final polish.
