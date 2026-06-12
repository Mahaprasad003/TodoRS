<script lang="ts">
  import { inboxTasks } from '$lib/stores/tasks';
  import { activeProjects } from '$lib/stores/projects';
  import { removeProject } from '$lib/stores/projects';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import TaskList from '$lib/components/shared/TaskList.svelte';
  import QuickAdd from '$lib/components/shared/QuickAdd.svelte';
  import { confirm } from '$lib/stores/confirm';

  // Read project filter from query param
  $: projectFilter = $page.url.searchParams.get('project');

  // Find project name for display
  $: activeProj = $activeProjects;
  $: selectedProject = projectFilter ? activeProj.find(p => p.id === projectFilter) : null;

  // Filter tasks by project when query param is present
  $: filteredTasks = projectFilter
    ? $inboxTasks.filter(t => t.project_id === projectFilter)
    : $inboxTasks;

  function clearFilter() {
    goto('/inbox');
  }
</script>

<svelte:head>
  <title>Inbox — TodoRS</title>
</svelte:head>

<div class="page-container">
  <div class="page-header">
    {#if selectedProject}
      <div class="page-header-row">
        <div>
          <div class="breadcrumb">
            <button class="breadcrumb-link" on:click={clearFilter}>Inbox</button>
            <span class="breadcrumb-sep">/</span>
            <span class="breadcrumb-current">{selectedProject.name}</span>
          </div>
          <p class="page-subtitle">{filteredTasks.length} pending {filteredTasks.length === 1 ? 'task' : 'tasks'}</p>
        </div>
        <button
          type="button"
          class="btn btn-danger"
          on:click={async () => {
            const ok = await confirm({ title: 'Delete project', message: 'Delete "' + selectedProject.name + '"? All tasks in this project will remain but become unassigned.', confirmLabel: 'Delete', danger: true });
            if (ok) {
              const pid = projectFilter!;
              removeProject(pid);
              goto('/inbox');
            }
          }}
          aria-label="Delete project"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <path d="M3 4H13M5 4V3C5 2.44772 5.44772 2 6 2H10C10.5523 2 11 2.44772 11 3V4M7 7V11M9 7V11M4 4L4.44721 13.3416C4.48224 14.2685 5.24508 15 6.17333 15H9.82667C10.7549 15 11.5178 14.2685 11.5528 13.3416L12 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Delete
        </button>
      </div>
    {:else}
      <h1 class="page-title">Inbox</h1>
      <p class="page-subtitle">Capture new tasks quickly</p>
    {/if}
  </div>

  <QuickAdd projectId={projectFilter || null} />

  <TaskList
    tasks={filteredTasks}
    emptyTitle={selectedProject ? 'No tasks in this project' : 'Inbox is clear'}
    emptyText={selectedProject ? 'Add a task to this project from the + button.' : 'Add a task from the + button or sync to pull tasks from other devices.'}
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

  .page-header-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-xs);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-family: var(--font-display);
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-headline);
    color: var(--color-ink);
    margin-bottom: var(--space-xs);
  }

  .breadcrumb-link {
    background: none;
    border: none;
    color: var(--color-ink-subtle);
    font-family: inherit;
    font-size: inherit;
    font-weight: inherit;
    letter-spacing: inherit;
    cursor: pointer;
    padding: 0;
    transition: color 0.15s ease;
  }

  .breadcrumb-link:hover {
    color: var(--color-primary);
    text-decoration: underline;
  }

  .breadcrumb-sep {
    color: var(--color-ink-subtle);
  }

  .breadcrumb-current {
    color: var(--color-ink);
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
