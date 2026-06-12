<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore, initAuth, signOut } from '$lib/stores/auth';
  import { loadTasks } from '$lib/stores/tasks';
  import { loadProjects } from '$lib/stores/projects';
  import { bootstrapAfterAuth, setAuthToken, setCurrentUserId, getCurrentUserId, sync } from '$lib/sync/client';
  import { syncStore } from '$lib/stores/sync';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import SyncStatus from '$lib/components/shared/SyncStatus.svelte';
  import Navigation from '$lib/components/shared/Navigation.svelte';
  import FloatingAddButton from '$lib/components/shared/FloatingAddButton.svelte';
  import AddTaskModal from '$lib/components/shared/AddTaskModal.svelte';
  import SearchOverlay from '$lib/components/shared/SearchOverlay.svelte';
  import { openSearch } from '$lib/stores/search';
  import { editingTask, closeEdit } from '$lib/stores/edit';
  import '../app.css';

  let showAddModal = false;
  let syncInterval: ReturnType<typeof setInterval> | null = null;

  // Open modal in edit mode when a task is tapped
  $: if ($editingTask) {
    showAddModal = true;
  }

  onMount(() => {
    initAuth();

    // Also sync when page regains focus (user switches back after editing in TUI)
    const handleVisibility = () => {
      if (document.visibilityState === 'visible' && $authStore.user) {
        sync().catch(() => {});
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);

    // Global keyboard shortcuts
    function handleKeydown(e: KeyboardEvent) {
      // Skip if user is typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === '/' && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        openSearch();
      }
    }
    document.addEventListener('keydown', handleKeydown);

    return () => {
      document.removeEventListener('visibilitychange', handleVisibility);
      document.removeEventListener('keydown', handleKeydown);
      if (syncInterval) clearInterval(syncInterval);
    };
  });

  // ── Bootstrap on auth state change ──
  $: user = $authStore.user;
  $: loading = $authStore.loading;
  $: session = $authStore.session;

  // When user becomes available, bootstrap sync and start periodic sync
  $: if (user && session && !loading) {
    bootstrapAfterAuth(user.id, session.access_token).catch(() => {});
    loadTasks(user.id);
    loadProjects(user.id);

    // Start periodic sync every 30s (matching TUI behavior) to pick up changes from other devices
    if (!syncInterval) {
      syncInterval = setInterval(() => {
        sync().catch(() => {});
      }, 30000);
    }
  }

  // ── Reload stores after sync completes ──
  $: if ($syncStore.status === 'synced' && $syncStore.lastSyncedAt && getCurrentUserId()) {
    const uid = getCurrentUserId()!;
    loadTasks(uid);
    loadProjects(uid);
  }

  // ── Auth guard routing ──
  $: if (!loading && !user && $page.url.pathname !== '/login') {
    goto('/login');
  }

  $: if (!loading && user && $page.url.pathname === '/login') {
    goto('/inbox');
  }

  async function handleSignOut() {
    await signOut();
    setAuthToken(null);
    setCurrentUserId(null);
    goto('/login');
  }

  $: currentPath = $page.url.pathname;

  const sidebarItems = [
    { href: '/inbox', label: 'Inbox', icon: 'inbox' },
    { href: '/today', label: 'Today', icon: 'today' },
    { href: '/upcoming', label: 'Upcoming', icon: 'upcoming' },
    { href: '/projects', label: 'Projects', icon: 'projects' },
    { href: '/completed', label: 'Completed', icon: 'completed' },
  ];

  const sidebarActions = [
    { label: 'Search', icon: 'search', action: () => openSearch(), shortcut: '/' },
  ];

  function handleSidebarAction(action: typeof sidebarActions[number]) {
    action.action();
  }
</script>

{#if loading}
  <div class="loading-screen">
    <div class="loading-spinner"></div>
    <div class="loading-text">Loading TodoRS...</div>
  </div>
{:else if !user}
  <slot />
{:else}
  <div class="app-layout">
    <!-- Desktop Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="sidebar-logo">TodoRS</div>
      </div>

      <nav class="sidebar-nav" aria-label="Sidebar navigation">
        {#each sidebarItems as item}
          <a
            href={item.href}
            class="sidebar-nav-item"
            class:active={currentPath === item.href}
          >
            <div class="sidebar-nav-icon">
              {#if item.icon === 'inbox'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M3 12H7L10 8L14 14L17 10L21 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <path d="M3 4V20H21V4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              {:else if item.icon === 'today'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <rect x="3" y="4" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2"/>
                  <path d="M3 10H21" stroke="currentColor" stroke-width="2"/>
                  <circle cx="12" cy="16" r="2" fill="currentColor"/>
                </svg>
              {:else if item.icon === 'upcoming'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <rect x="3" y="4" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2"/>
                  <path d="M3 10H21" stroke="currentColor" stroke-width="2"/>
                  <path d="M8 2V6M16 2V6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                  <path d="M12 14V18M14 16H10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
              {:else if item.icon === 'projects'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M3 7V5C3 3.89543 3.89543 3 5 3H9L11 6H19C20.1046 6 21 6.89543 21 8V19C21 20.1046 20.1046 21 19 21H5C3.89543 21 3 20.1046 3 19V7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              {:else if item.icon === 'completed'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <rect x="3" y="4" width="18" height="16" rx="2" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M8 12L11 15L16 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              {/if}
            </div>
            <span>{item.label}</span>
          </a>
        {/each}
      </nav>

      <!-- Sidebar action items -->
      <nav class="sidebar-actions" aria-label="Sidebar actions">
        {#each sidebarActions as action}
          <button
            type="button"
            class="sidebar-nav-item"
            on:click={() => handleSidebarAction(action)}
          >
            <div class="sidebar-nav-icon">
              {#if action.icon === 'search'}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2"/>
                  <path d="M16 16L21 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
              {/if}
            </div>
            <span>{action.label}</span>
            {#if action.shortcut}
              <span class="sidebar-shortcut">{action.shortcut}</span>
            {/if}
          </button>
        {/each}
      </nav>

      <div class="sidebar-footer">
        <SyncStatus />
        <button class="btn btn-tertiary sidebar-signout" on:click={handleSignOut}>
          Sign out
        </button>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="main-content">
      <header class="top-nav">
        <div class="top-nav-left">
          <span class="sync-indicator-mobile">
            <SyncStatus />
          </span>
        </div>
        <div class="top-nav-right">
          <span class="current-user">{ user?.email }</span>
          <button class="btn btn-tertiary" on:click={handleSignOut}>Sign out</button>
        </div>
      </header>

      <slot />
    </main>
  </div>

  <!-- Floating Action Button (mobile only) -->
  <FloatingAddButton on:open={() => { closeEdit(); showAddModal = true; }} />

  <!-- Add Task Modal (create or edit) -->
  <AddTaskModal
    bind:show={showAddModal}
    task={$editingTask}
    on:close={() => { showAddModal = false; closeEdit(); }}
  />

  <!-- Search Overlay (global) -->
  <SearchOverlay />

  <!-- Mobile Bottom Navigation -->
  <Navigation />
{/if}

<style>
  .app-layout {
    display: flex;
    min-height: 100dvh;
  }

  .sidebar {
    width: 240px;
    border-right: 1px solid var(--color-hairline);
    padding: var(--space-md);
    background-color: var(--color-canvas);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
  }

  @media (max-width: 767px) {
    .sidebar {
      display: none;
    }
  }

  .sidebar-header {
    padding: var(--space-xs) 0;
  }

  .sidebar-logo {
    font-family: var(--font-display);
    font-size: var(--text-card-title);
    font-weight: var(--weight-card-title);
    letter-spacing: var(--tracking-card-title);
    color: var(--color-ink);
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-xxs);
    margin-top: var(--space-lg);
  }

  .sidebar-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-xxs);
    margin-top: var(--space-xs);
    padding-bottom: var(--space-md);
    border-bottom: 1px solid var(--color-hairline);
    flex: 1;
  }

  .sidebar-nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    text-decoration: none;
    color: var(--color-ink-subtle);
    font-size: var(--text-body-sm);
    transition: all 0.15s ease;
  }

  .sidebar-nav-item:hover {
    background-color: var(--color-surface-1);
    color: var(--color-ink);
  }

  .sidebar-nav-item.active {
    background-color: var(--color-surface-1);
    color: var(--color-primary);
  }

  .sidebar-shortcut {
    margin-left: auto;
    font-size: var(--text-caption);
    color: var(--color-ink-tertiary);
    font-family: var(--font-mono);
  }

  .sidebar-nav-icon {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sidebar-footer {
    padding-top: var(--space-md);
    border-top: 1px solid var(--color-hairline);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .sidebar-signout {
    text-align: left;
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
  }

  .main-content {
    flex: 1;
    min-width: 0;
  }

  .top-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 var(--space-md);
    background-color: var(--color-canvas);
    border-bottom: 1px solid var(--color-hairline);
  }

  .top-nav-left {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .top-nav-right {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  @media (max-width: 767px) {
    .top-nav-right .current-user,
    .top-nav-right .btn-tertiary {
      display: none;
    }
  }

  .current-user {
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
  }

  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100dvh;
    gap: var(--space-md);
    background-color: var(--color-canvas);
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-hairline);
    border-top-color: var(--color-primary);
    border-radius: 9999px;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .loading-text {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
  }
</style>
