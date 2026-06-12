<script lang="ts">
  import { todayTasks, addTask, completedTodayTasks } from '$lib/stores/tasks';
  import TaskList from '$lib/components/shared/TaskList.svelte';
  import QuickAdd from '$lib/components/shared/QuickAdd.svelte';

  let todayDate = '';
  $: {
    const d = new Date();
    todayDate = d.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' });
  }
</script>

<svelte:head>
  <title>Today — TodoRS</title>
</svelte:head>

<div class="page-container">
  <div class="page-header">
    <h1 class="page-title">Today</h1>
    <p class="page-subtitle">{todayDate}</p>
  </div>

  <QuickAdd />

  <TaskList
    tasks={$todayTasks}
    emptyTitle="No tasks due today"
    emptyText="Add a task above or check your inbox for pending items."
  />

  <hr class="divider" />

  <div class="section-header">
    <h2 class="section-title">Completed Today</h2>
  </div>

  {#if $completedTodayTasks.length === 0}
    <p class="body-sm" style="color: var(--color-ink-subtle); text-align: center; padding: var(--space-lg);">
      No tasks completed today yet.
    </p>
  {:else}
    <TaskList tasks={$completedTodayTasks} />
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

  .divider {
    border: none;
    border-top: 1px solid var(--color-hairline);
    margin: var(--space-lg) 0;
  }

  .section-header {
    margin-bottom: var(--space-md);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: var(--text-card-title);
    font-weight: var(--weight-card-title);
    letter-spacing: var(--tracking-card-title);
    color: var(--color-ink-muted);
  }
</style>
