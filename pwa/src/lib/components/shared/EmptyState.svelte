<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let title: string;
  export let message: string;

  const dispatch = createEventDispatcher();
</script>

<div class="empty-state">
  <div class="empty-state-icon">
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <rect x="3" y="4" width="18" height="16" rx="2" stroke="currentColor" stroke-width="2"/>
      <path d="M8 12L11 15L16 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </div>
  <div class="empty-state-title">{title}</div>
  <div class="empty-state-text">{message}</div>
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-xxl) var(--space-lg);
    text-align: center;
    gap: var(--space-md);
  }

  .empty-state-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    background-color: var(--color-surface-1);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-ink-subtle);
  }

  .empty-state-title {
    font-family: var(--font-display);
    font-size: var(--text-card-title);
    font-weight: var(--weight-card-title);
    letter-spacing: var(--tracking-card-title);
    color: var(--color-ink-muted);
  }

  .empty-state-text {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
    max-width: 300px;
  }
</style>
