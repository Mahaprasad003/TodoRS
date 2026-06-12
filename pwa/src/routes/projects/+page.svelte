<script lang="ts">
  import { activeProjects, addProject } from '$lib/stores/projects';
  import { tasksStore } from '$lib/stores/tasks';
  import { goto } from '$app/navigation';

  let showNewForm = false;
  let newName = '';
  let newColor = '#5e6ad2';
  let error = '';

  const colorOptions = [
    '#5e6ad2', '#27a644', '#f5a623', '#ee0000',
    '#50e3c2', '#7928ca', '#ff0080', '#f7f8f8',
  ];

  $: taskCounts = $tasksStore.reduce((acc, t) => {
    if (t.project_id && t.status === 'pending' && !t.deleted_at) {
      acc[t.project_id] = (acc[t.project_id] || 0) + 1;
    }
    return acc;
  }, {} as Record<string, number>);

  function getTaskCount(projectId: string): number {
    return taskCounts[projectId] || 0;
  }

  async function handleCreate() {
    const name = newName.trim();
    if (!name) return;
    error = '';
    try {
      await addProject(name, newColor);
      newName = '';
      newColor = '#5e6ad2';
      showNewForm = false;
    } catch (e: any) {
      error = e.message || 'Failed to create project';
    }
  }
</script>

<svelte:head>
  <title>Projects — TodoRS</title>
</svelte:head>

<div class="page-container">
  <div class="page-header">
    <div class="page-header-row">
      <div>
        <h1 class="page-title">Projects</h1>
        <p class="page-subtitle">Organize tasks by project</p>
      </div>
      <button class="btn btn-primary" on:click={() => (showNewForm = !showNewForm)}>
        {showNewForm ? 'Cancel' : 'New Project'}
      </button>
    </div>
  </div>

  {#if showNewForm}
    <div class="new-project-card">
      <h3 class="new-project-title">Create Project</h3>
      {#if error}
        <div class="form-error">{error}</div>
      {/if}
      <div class="new-project-form">
        <input type="text" class="input" bind:value={newName} placeholder="Project name" required />
        <div class="color-picker">
          <span class="color-label">Color</span>
          <div class="color-options">
            {#each colorOptions as color}
              <button type="button" class="color-swatch" class:selected={newColor === color} style="background-color: {color};" on:click={() => (newColor = color)} aria-label="Color {color}"></button>
            {/each}
          </div>
        </div>
        <button class="btn btn-primary" on:click={handleCreate} disabled={!newName.trim()}>Create Project</button>
      </div>
    </div>
  {/if}

  {#if $activeProjects.length === 0}
    <div class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
          <path d="M3 7V5C3 3.89543 3.89543 3 5 3H9L11 6H19C20.1046 6 21 6.89543 21 8V19C21 20.1046 20.1046 21 19 21H5C3.89543 21 3 20.1046 3 19V7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <p class="empty-title">No projects yet</p>
      <p class="empty-text">Create a project to organize your tasks.</p>
    </div>
  {:else}
    <div class="projects-list">
      {#each $activeProjects as project (project.id)}
        <button type="button" class="project-card" on:click={() => goto('/inbox?project=' + project.id)}>
          <div class="project-left">
            <div class="project-color" style="background-color: {project.color || '#5e6ad2'};}"></div>
            <div>
              <div class="project-name">{project.name}</div>
              <div class="project-count">{getTaskCount(project.id)} {getTaskCount(project.id) === 1 ? 'task' : 'tasks'}</div>
            </div>
          </div>
          <div class="project-arrow">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
        </button>
      {/each}
    </div>
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

  .page-header-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
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

  .new-project-card {
    background-color: var(--color-surface-2);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-lg);
    margin-bottom: var(--space-lg);
  }

  .new-project-title {
    font-family: var(--font-display);
    font-size: var(--text-card-title);
    font-weight: var(--weight-card-title);
    letter-spacing: var(--tracking-card-title);
    color: var(--color-ink);
    margin-bottom: var(--space-md);
  }

  .new-project-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .form-error {
    background-color: var(--color-error-soft);
    color: var(--color-error);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    font-size: var(--text-caption);
  }

  .color-picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .color-label {
    font-size: var(--text-caption);
    color: var(--color-ink-muted);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .color-options {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 9999px;
    border: 2px solid transparent;
    cursor: pointer;
    transition: border-color 0.15s ease;
    padding: 0;
  }

  .color-swatch:hover { border-color: var(--color-ink-muted); }
  .color-swatch.selected { border-color: var(--color-ink); }

  .projects-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .project-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--color-surface-1);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-md);
    transition: background-color 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
    color: inherit;
    font-family: inherit;
    font-size: inherit;
  }

  .project-card:hover {
    background-color: var(--color-surface-2);
  }

  .project-left {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .project-color {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .project-name {
    font-size: var(--text-body);
    font-weight: 500;
    color: var(--color-ink);
  }

  .project-count {
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
    margin-top: 2px;
  }

  .project-arrow {
    color: var(--color-ink-subtle);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-xxl) var(--space-lg);
    text-align: center;
  }

  .empty-icon { color: var(--color-ink-subtle); margin-bottom: var(--space-md); }
  .empty-title { font-size: var(--text-body); font-weight: 500; color: var(--color-ink); margin-bottom: var(--space-xs); }
  .empty-text { font-size: var(--text-body-sm); color: var(--color-ink-subtle); }
</style>
