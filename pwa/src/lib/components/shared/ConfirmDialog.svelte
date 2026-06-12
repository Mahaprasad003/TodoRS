<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let show = false;
  export let title = 'Confirm';
  export let message = 'Are you sure?';
  export let confirmLabel = 'Confirm';
  export let cancelLabel = 'Cancel';
  export let danger = false;

  const dispatch = createEventDispatcher();

  function handleConfirm() {
    show = false;
    dispatch('confirm');
  }

  function handleCancel() {
    show = false;
    dispatch('cancel');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleCancel();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="backdrop"
    on:click|self={handleCancel}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <div class="dialog">
      <div class="dialog-icon" class:danger>
        {#if danger}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5"/>
            <path d="M12 8V13M12 16V16.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        {:else}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5"/>
            <path d="M12 8V13M12 16V16.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        {/if}
      </div>
      <h3 class="dialog-title">{title}</h3>
      <p class="dialog-message">{message}</p>
      <div class="dialog-actions">
        <button type="button" class="btn btn-tertiary" on:click={handleCancel}>{cancelLabel}</button>
        <button
          type="button"
          class="btn"
          class:btn-danger={danger}
          class:btn-primary={!danger}
          on:click={handleConfirm}
        >{confirmLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.6);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.12s ease;
    padding: var(--space-lg);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .dialog {
    width: 100%;
    max-width: 360px;
    background-color: var(--color-surface-1);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-xl);
    animation: scaleIn 0.12s ease;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    text-align: center;
  }

  @keyframes scaleIn {
    from { transform: scale(0.95); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  .dialog-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    background-color: var(--color-surface-2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-ink-subtle);
  }

  .dialog-icon.danger {
    color: var(--color-error);
    background-color: var(--color-error-soft);
  }

  .dialog-title {
    font-family: var(--font-display);
    font-size: var(--text-card-title);
    font-weight: var(--weight-card-title);
    letter-spacing: var(--tracking-card-title);
    color: var(--color-ink);
  }

  .dialog-message {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
    line-height: var(--lh-body);
  }

  .dialog-actions {
    display: flex;
    gap: var(--space-sm);
    width: 100%;
    margin-top: var(--space-xs);
  }

  .dialog-actions .btn {
    flex: 1;
    justify-content: center;
  }
</style>
