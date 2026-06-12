<script lang="ts">
  import { upcomingTasks } from '$lib/stores/tasks';
  import TaskList from '$lib/components/shared/TaskList.svelte';
  import QuickAdd from '$lib/components/shared/QuickAdd.svelte';
</script>

<svelte:head>
  <title>Upcoming — TodoRS</title>
</svelte:head>

<div class="page-container">
  <div class="page-header">
    <h1 class="page-title">Upcoming</h1>
    <p class="page-subtitle">Tasks with future due dates</p>
  </div>

  <QuickAdd />

  <TaskList
    tasks={$upcomingTasks}
    emptyTitle="No upcoming tasks"
    emptyText="Tasks with future due dates will appear here."
  />
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
</style>
