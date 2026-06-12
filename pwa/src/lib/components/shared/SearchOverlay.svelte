<script lang="ts">
  import { searchOpen, searchQuery, searchResults, closeSearch } from '$lib/stores/search';
  import TaskItem from './TaskItem.svelte';
  import { onMount } from 'svelte';

  let inputEl: HTMLInputElement;
  let overlayEl: HTMLDivElement;

  $: if ($searchOpen && inputEl) {
    setTimeout(() => inputEl.focus(), 50);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeSearch();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === overlayEl) {
      closeSearch();
    }
  }
</script>

{#if $searchOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="search-backdrop"
    bind:this={overlayEl}
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-label="Search tasks"
    tabindex="-1"
  >
    <div class="search-panel">
      <div class="search-input-row">
        <svg class="search-icon" width="18" height="18" viewBox="0 0 24 24" fill="none">
          <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2"/>
          <path d="M16 16L21 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <input
          bind:this={inputEl}
          type="text"
          class="search-input"
          bind:value={$searchQuery}
          placeholder="Search tasks by title or description..."
          on:keydown={handleKeydown}
          aria-label="Search tasks"
        />
        <button class="search-close" on:click={closeSearch} aria-label="Close search">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 4L12 12M12 4L4 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="search-results">
        {#if $searchQuery.trim() && $searchResults.length === 0}
          <div class="search-empty">
            <div class="search-empty-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2"/>
                <path d="M16 16L21 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </div>
            <div class="search-empty-text">No tasks match "{ $searchQuery }"</div>
          </div>
        {:else if $searchResults.length > 0}
          <div class="results-count">{ $searchResults.length } { $searchResults.length === 1 ? 'task' : 'tasks' }</div>
          <div class="results-list">
            {#each $searchResults as task (task.id)}
              <TaskItem {task} />
            {/each}
          </div>
        {:else if !$searchQuery.trim()}
          <div class="search-hint">
            <div class="search-hint-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <path d="M15 4V8H19" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M15 4H6C5.44772 4 5 4.44772 5 5V19C5 19.5523 5.44772 20 6 20H18C18.5523 20 19 19.5523 19 19V8L15 4Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M9 14H15M12 11V17" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </div>
            <p class="search-hint-text">Type to search across all tasks</p>
            <p class="search-hint-shortcut">Press <kbd>Esc</kbd> to close</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .search-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.5);
    z-index: 300;
    display: flex;
    justify-content: center;
    padding-top: 80px;
    animation: fadeIn 0.1s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .search-panel {
    width: 100%;
    max-width: 600px;
    max-height: calc(100dvh - 120px);
    background-color: var(--color-surface-2);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideDown 0.12s ease;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  @keyframes slideDown {
    from { transform: translateY(-10px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .search-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    border-bottom: 1px solid var(--color-hairline);
  }

  .search-icon {
    color: var(--color-ink-subtle);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: var(--text-body-lg);
    color: var(--color-ink);
    font-family: var(--font-body);
  }

  .search-input::placeholder {
    color: var(--color-ink-tertiary);
  }

  .search-close {
    background: none;
    border: none;
    color: var(--color-ink-subtle);
    cursor: pointer;
    padding: var(--space-xxs);
    border-radius: var(--radius-xs);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s ease;
  }

  .search-close:hover {
    color: var(--color-ink);
  }

  .search-results {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-xs) 0;
  }

  .results-count {
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
    padding: var(--space-xs) var(--space-md);
  }

  .results-list {
    display: flex;
    flex-direction: column;
  }

  .search-empty,
  .search-hint {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xxl) var(--space-lg);
    text-align: center;
  }

  .search-empty-icon,
  .search-hint-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    background-color: var(--color-surface-1);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-ink-subtle);
  }

  .search-empty-text {
    font-size: var(--text-body);
    color: var(--color-ink-muted);
  }

  .search-hint-text {
    font-size: var(--text-body);
    color: var(--color-ink-muted);
  }

  .search-hint-shortcut {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
  }

  .search-hint-shortcut kbd {
    display: inline-block;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    background-color: var(--color-surface-3);
    border: 1px solid var(--color-hairline);
    font-family: var(--font-mono);
    font-size: var(--text-caption);
  }
</style>
