<script lang="ts">
  import { completedTasks } from '$lib/stores/tasks';
  import TaskList from '$lib/components/shared/TaskList.svelte';

  $: grouped = groupByDate($completedTasks);

  interface Group {
    label: string;
    tasks: import('$lib/db/schema').TaskRecord[];
  }

  function groupByDate(tasks: import('$lib/db/schema').TaskRecord[]): Group[] {
    const today = new Date();
    const todayStr = today.toISOString().slice(0, 10);
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    const yesterdayStr = yesterday.toISOString().slice(0, 10);

    const groups: Record<string, Group> = {};

    for (const t of tasks) {
      const completedDate = t.completed_at ? t.completed_at.slice(0, 10) : t.updated_at.slice(0, 10);
      let label: string;
      if (completedDate === todayStr) label = 'Today';
      else if (completedDate === yesterdayStr) label = 'Yesterday';
      else {
        const d = new Date(completedDate);
        const diffDays = Math.floor((today.getTime() - d.getTime()) / (1000 * 60 * 60 * 24));
        label = diffDays <= 7 ? d.toLocaleDateString([], { weekday: 'long' }) : d.toLocaleDateString([], { month: 'long', day: 'numeric' });
      }

      if (!groups[label]) groups[label] = { label, tasks: [] };
      groups[label].tasks.push(t);
    }

    // Maintain sort order: today, yesterday, then reverse chronological
    const order: string[] = [];
    if (groups['Today']) order.push('Today');
    if (groups['Yesterday']) order.push('Yesterday');

    const rest = Object.keys(groups).filter(k => k !== 'Today' && k !== 'Yesterday').sort((a, b) => {
      // Parse dates for comparison
      const dateA = groups[a].tasks[0]?.completed_at || groups[a].tasks[0]?.updated_at || '';
      const dateB = groups[b].tasks[0]?.completed_at || groups[b].tasks[0]?.updated_at || '';
      return dateB.localeCompare(dateA);
    });
    order.push(...rest);

    return order.map(k => groups[k]);
  }
</script>

<svelte:head>
  <title>Completed — TodoRS</title>
</svelte:head>

<div class="page-container">
  <div class="page-header">
    <h1 class="page-title">Completed</h1>
    <p class="page-subtitle">Archive of completed tasks</p>
  </div>

  {#if $completedTasks.length === 0}
    <div class="empty-state">
      <div class="empty-state-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
          <rect x="3" y="4" width="18" height="16" rx="2" stroke="currentColor" stroke-width="1.5"/>
          <path d="M8 12L11 15L16 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <p class="empty-state-title">No completed tasks yet</p>
      <p class="empty-state-text">Tasks you complete will appear here.</p>
    </div>
  {:else}
    <div class="summary-row">
      <span class="summary-count">{$completedTasks.length} completed</span>
    </div>

    {#each grouped as group}
      <div class="group">
        <div class="group-header">
          <h2 class="group-title">{group.label}</h2>
          <span class="group-count">{group.tasks.length}</span>
        </div>
        <TaskList tasks={group.tasks} />
      </div>
    {/each}
  {/if}
</div>

<style>
  .page-container {
    max-width: 800px;
    margin: 0 auto;
    padding: var(--space-md);
    padding-bottom: calc(64px + var(--space-md));
  }

  @media (min-width: 768px) {
    .page-container {
      padding: var(--space-lg);
      padding-bottom: var(--space-lg);
    }
  }

  .page-header {
    margin-bottom: var(--space-lg);
  }

  .page-title {
    font-family: var(--font-display);
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-headline);
    color: var(--color-ink);
    margin-bottom: var(--space-xs);
  }

  .page-subtitle {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
  }

  .summary-row {
    margin-bottom: var(--space-md);
  }

  .summary-count {
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
  }

  .group {
    margin-bottom: var(--space-lg);
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-xs);
    padding: 0 var(--space-md);
  }

  .group-title {
    font-family: var(--font-display);
    font-size: var(--text-subhead);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-subhead);
    color: var(--color-ink-muted);
  }

  .group-count {
    font-size: var(--text-caption);
    color: var(--color-ink-tertiary);
    font-family: var(--font-mono);
    background-color: var(--color-surface-2);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
  }

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
    width: 64px;
    height: 64px;
    border-radius: var(--radius-lg);
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
