<script lang="ts">
  import { syncStore } from '$lib/stores/sync';
  import { sync } from '$lib/sync/client';

  $: status = $syncStore.status;
  $: error = $syncStore.error;
  $: lastSyncedAt = $syncStore.lastSyncedAt;

  function formatTime(iso: string | null): string {
    if (!iso) return '';
    const d = new Date(iso);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  $: statusText = status === 'syncing' ? 'Syncing...'
    : status === 'synced' ? `Synced ${formatTime(lastSyncedAt)}`
    : status === 'error' ? `Sync error`
    : status === 'offline' ? 'Offline'
    : '';

  let syncing = false;

  async function handleSync() {
    if (syncing) return;
    syncing = true;
    try {
      await sync();
    } finally {
      syncing = false;
    }
  }
</script>

<div class="sync-indicator">
  <span class="status-dot {status}"></span>
  <span class="status-text">{statusText}</span>
  {#if status === 'error' && error}
    <span class="error-text" title={error}>!</span>
  {/if}
  <button
    class="sync-btn"
    on:click={handleSync}
    disabled={syncing || status === 'syncing'}
    aria-label="Sync now"
    title="Sync now"
  >
    <svg
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      class:spinning={syncing || status === 'syncing'}
    >
      <path d="M2 8C2 4.68629 4.68629 2 8 2C10.5 2 12.5 3.5 13.5 5.5M14 8C14 11.3137 11.3137 14 8 14C5.5 14 3.5 12.5 2.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M10 5.5H13.5V2M6 10.5H2.5V14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </button>
</div>

<style>
  .sync-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-xxs);
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .status-dot.synced { background-color: var(--color-success); }
  .status-dot.syncing { background-color: var(--color-warning); animation: pulse 1s infinite; }
  .status-dot.error { background-color: var(--color-error); }
  .status-dot.offline { background-color: var(--color-ink-subtle); }
  .status-dot.idle { background-color: var(--color-ink-subtle); }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .error-text {
    color: var(--color-error);
    font-weight: 600;
    cursor: help;
  }

  .sync-btn {
    background: none;
    border: none;
    color: var(--color-ink-subtle);
    cursor: pointer;
    padding: 2px;
    border-radius: var(--radius-xs);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s ease;
  }

  .sync-btn:hover {
    color: var(--color-primary);
  }

  .sync-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .sync-btn .spinning {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
