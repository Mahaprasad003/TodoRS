<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { addTask, updateTaskField } from '$lib/stores/tasks';
  import { activeProjects } from '$lib/stores/projects';
  import type { TaskRecord } from '$lib/db/schema';

  export let show = false;
  /** When set, the modal operates in edit mode instead of create mode. */
  export let task: TaskRecord | null = null;

  const dispatch = createEventDispatcher();

  let title = '';
  let selectedProjectId = '';
  let selectedProjectName = '';
  let dueDate = '';
  let dueTime = '';
  let priority: 'none' | 'low' | 'medium' | 'high' | 'urgent' = 'none';
  let repeat: 'none' | 'daily' | 'weekly' | 'monthly' | 'yearly' = 'none';
  let titleInput: HTMLInputElement;
  let submitting = false;
  let activePicker: 'project' | 'date' | 'priority' | 'repeat' | null = null;

  const priorityLabels: Record<string, string> = {
    none: 'None', low: 'Low', medium: 'Medium', high: 'High', urgent: 'Urgent',
  };
  const repeatLabels: Record<string, string> = {
    none: 'Never', daily: 'Daily', weekly: 'Weekly', monthly: 'Monthly', yearly: 'Yearly',
  };
  const priorities = ['none', 'low', 'medium', 'high', 'urgent'] as const;
  const repeats = ['none', 'daily', 'weekly', 'monthly', 'yearly'] as const;

  // ── Pre-fill fields when editing a task ──
  $: if (task && show) {
    title = task.title || '';
    selectedProjectId = task.project_id || '';
    dueDate = task.due_at ? task.due_at.slice(0, 10) : '';
    // Extract time from due_at if it has a non-midnight time component
    if (task.due_at && task.due_at.includes('T')) {
      const timePart = task.due_at.slice(11, 16);
      dueTime = timePart !== '00:00' ? timePart : '';
    } else {
      dueTime = '';
    }
    priority = task.priority || 'none';
    // Map recurrence from task's recurrence_rule_id — we can't know the rule details
    // from the task alone, so default to 'none'. Full recurring-edit is a future concern.
    repeat = 'none';
  } else if (!show) {
    // Reset when modal closes (but keep task pre-fill ready for next open)
    // We reset in close() and after submit
  }

  $: isEditing = task !== null;

  onMount(() => {
    if (titleInput) setTimeout(() => titleInput.focus(), 200);
  });

  $: if (show && titleInput) {
    setTimeout(() => titleInput.focus(), 200);
    activePicker = null;
  }

  // Update project name when ID changes
  $: {
    const p = $activeProjects.find(proj => proj.id === selectedProjectId);
    selectedProjectName = p?.name || '';
  }

  /** Build a full ISO datetime string from dueDate + dueTime, or null if empty. */
  function buildDueAt(): string | null {
    if (!dueDate) return null;
    if (dueTime) return `${dueDate}T${dueTime}:00.000Z`;
    return `${dueDate}T00:00:00.000Z`;
  }

  function resetForm() {
    title = '';
    selectedProjectId = '';
    dueDate = '';
    dueTime = '';
    priority = 'none';
    repeat = 'none';
  }

  function close() {
    show = false;
    activePicker = null;
    if (!isEditing) resetForm();
    dispatch('close');
  }

  function openPicker(picker: 'project' | 'date' | 'priority' | 'repeat') {
    activePicker = picker;
  }

  function closePicker() {
    activePicker = null;
  }

  async function handleSubmit() {
    const trimmed = title.trim();
    if (!trimmed || submitting) return;
    submitting = true;
    try {
      if (isEditing && task) {
        // ── Edit mode: update existing task ──
        const updates: Partial<TaskRecord> = {
          title: trimmed,
        };
        if (selectedProjectId) updates.project_id = selectedProjectId;
        else updates.project_id = undefined;
        const resolvedDueAt = buildDueAt();
        updates.due_at = resolvedDueAt || undefined;
        updates.priority = priority;

        await updateTaskField(task.id, updates);
        dispatch('saved', { action: 'updated', title: trimmed });
      } else {
        // ── Create mode: add new task ──
        await addTask(trimmed, {
          project_id: selectedProjectId || undefined,
          priority,
          due_at: buildDueAt() || undefined,
          recurrence: repeat !== 'none' ? repeat : undefined,
        });
      }
      resetForm();
      close();
    } finally {
      submitting = false;
    }
  }

  function pickerLabel(label: string): string {
    return label === 'none' ? 'None' : label.charAt(0).toUpperCase() + label.slice(1);
  }
</script>

{#if show}
  <!-- Main Modal -->
  <div
    class="backdrop"
    on:click|self={activePicker ? closePicker : close}
    on:keydown={(e) => { if (e.key === 'Escape') { activePicker ? closePicker() : close(); } }}
    role="dialog"
    aria-modal="true"
    aria-label={isEditing ? 'Edit task' : 'Add task'}
    tabindex="-1"
  >
    <div class="modal" class:picker-open={activePicker !== null} role="presentation">
      <div class="modal-handle"></div>

      {#if activePicker}
        <!-- ─── PICKER VIEW ─── -->
        <div class="picker-header">
          <button type="button" class="picker-back" on:click={closePicker} aria-label="Go back">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <path d="M13 4L7 10L13 16" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <h2 class="picker-title">
            {#if activePicker === 'project'}Project
            {:else if activePicker === 'date'}Due Date
            {:else if activePicker === 'priority'}Priority
            {:else}Repeat{/if}
          </h2>
        </div>

        <div class="picker-body">
          {#if activePicker === 'project'}
            <button
              type="button"
              class="picker-option"
              class:selected={selectedProjectId === ''}
              on:click={() => { selectedProjectId = ''; closePicker(); }}
            >No project</button>
            {#each $activeProjects as project (project.id)}
              <button
                type="button"
                class="picker-option"
                class:selected={selectedProjectId === project.id}
                on:click={() => { selectedProjectId = project.id; closePicker(); }}
              >
                <span class="picker-dot" style="background-color: {project.color || '#5e6ad2'};"></span>
                {project.name}
              </button>
            {/each}

          {:else if activePicker === 'date'}
            <input
              type="date"
              class="input date-input"
              bind:value={dueDate}
            />
            <input
              type="time"
              class="input time-input"
              bind:value={dueTime}
              aria-label="Time (optional)"
            />
            <div class="picker-actions">
              <button
                type="button"
                class="btn btn-tertiary"
                on:click={() => { dueDate = ''; dueTime = ''; closePicker(); }}
              >Clear</button>
              <button
                type="button"
                class="btn btn-primary"
                on:click={closePicker}
              >Done</button>
            </div>

          {:else if activePicker === 'priority'}
            {#each priorities as p}
              <button
                type="button"
                class="picker-option"
                class:selected={priority === p}
                class:urgent={p === 'urgent'}
                class:high={p === 'high'}
                on:click={() => { priority = p; closePicker(); }}
              >{pickerLabel(p)}</button>
            {/each}

          {:else if activePicker === 'repeat'}
            {#each repeats as r}
              <button
                type="button"
                class="picker-option"
                class:selected={repeat === r}
                on:click={() => { repeat = r; closePicker(); }}
              >{repeatLabels[r]}</button>
            {/each}
          {/if}
        </div>

      {:else}
        <!-- ─── MAIN FORM ─── -->
        <h2 class="modal-title">{isEditing ? 'Edit Task' : 'New Task'}</h2>

        <form class="add-form" on:submit|preventDefault={handleSubmit}>
          <input
            bind:this={titleInput}
            type="text"
            class="input title-input"
            bind:value={title}
            placeholder="What needs to be done?"
            required
            disabled={submitting}
          />

          <!-- Option Summary Rows -->
          <div class="option-rows">
            <button type="button" class="option-row" on:click={() => openPicker('project')} disabled={submitting}>
              <span class="option-label">Project</span>
              <span class="option-value" class:muted={!selectedProjectName}>
                {selectedProjectName || 'None'}
              </span>
              <svg class="option-chevron" width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M5 3.5L9 7L5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            <button type="button" class="option-row" on:click={() => openPicker('date')} disabled={submitting}>
              <span class="option-label">Due date</span>
              <span class="option-value" class:muted={!dueDate}>
                {dueDate
                  ? dueTime
                    ? `${new Date(dueDate).toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' })} ${dueTime}`
                    : new Date(dueDate).toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' })
                  : 'None'}
              </span>
              <svg class="option-chevron" width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M5 3.5L9 7L5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            <button type="button" class="option-row" on:click={() => openPicker('priority')} disabled={submitting}>
              <span class="option-label">Priority</span>
              <span class="option-value" class:muted={priority === 'none'}>
                {priorityLabels[priority]}
              </span>
              <svg class="option-chevron" width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M5 3.5L9 7L5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            <button type="button" class="option-row" on:click={() => openPicker('repeat')} disabled={submitting}>
              <span class="option-label">Repeat</span>
              <span class="option-value" class:muted={repeat === 'none'}>
                {repeatLabels[repeat]}
              </span>
              <svg class="option-chevron" width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M5 3.5L9 7L5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>

          <div class="actions">
            <button type="button" class="btn btn-tertiary" on:click={close} disabled={submitting}>Cancel</button>
            <button type="submit" class="btn btn-primary" disabled={!title.trim() || submitting}>
              {submitting ? 'Saving...' : isEditing ? 'Save' : 'Add Task'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.6);
    z-index: 200;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal {
    width: 100%;
    max-width: 500px;
    background-color: var(--color-surface-1);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
    padding: var(--space-md) var(--space-lg) var(--space-xl);
    animation: slideUp 0.2s ease;
    max-height: 85dvh;
    overflow-y: auto;
  }

  @keyframes slideUp {
    from { transform: translateY(100%); }
    to { transform: translateY(0); }
  }

  .modal-handle {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background-color: var(--color-hairline-strong);
    margin: 0 auto var(--space-md);
  }

  .modal-title {
    font-family: var(--font-display);
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-headline);
    color: var(--color-ink);
    margin-bottom: var(--space-lg);
  }

  /* ── Main Form ── */

  .add-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .title-input {
    font-size: var(--text-body-lg);
    padding: var(--space-sm) var(--space-md);
    height: 48px;
  }

  .option-rows {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .option-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-hairline);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background-color 0.1s ease;
  }

  .option-row:last-child {
    border-bottom: none;
  }

  .option-row:hover {
    background-color: var(--color-surface-2);
  }

  .option-label {
    font-size: var(--text-body-sm);
    color: var(--color-ink-muted);
    min-width: 72px;
  }

  .option-value {
    flex: 1;
    font-size: var(--text-body);
    color: var(--color-ink);
  }

  .option-value.muted {
    color: var(--color-ink-subtle);
  }

  .option-chevron {
    color: var(--color-ink-subtle);
    flex-shrink: 0;
  }

  .actions {
    display: flex;
    gap: var(--space-sm);
  }

  .actions .btn {
    flex: 1;
    justify-content: center;
  }

  /* ── Picker View ── */

  .picker-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-lg);
  }

  .picker-back {
    background: none;
    border: none;
    color: var(--color-ink-subtle);
    cursor: pointer;
    padding: var(--space-xxs);
    border-radius: var(--radius-sm);
    transition: color 0.15s ease;
  }

  .picker-back:hover {
    color: var(--color-ink);
  }

  .picker-title {
    font-family: var(--font-display);
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-headline);
    color: var(--color-ink);
  }

  .picker-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-xxs);
  }

  .picker-option {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: none;
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-md);
    color: var(--color-ink);
    font-size: var(--text-body);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: all 0.1s ease;
  }

  .picker-option:hover {
    background-color: var(--color-surface-2);
  }

  .picker-option.selected {
    border-color: var(--color-primary);
    background-color: rgba(94, 106, 210, 0.1);
  }

  .picker-option.urgent.selected {
    border-color: var(--color-error);
    background-color: rgba(238, 0, 0, 0.1);
  }

  .picker-option.high.selected {
    border-color: var(--color-warning);
    background-color: rgba(245, 166, 35, 0.1);
  }

  .picker-dot {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .date-input,
  .time-input {
    font-size: var(--text-body-lg);
    padding: var(--space-md);
    height: 48px;
  }

  .picker-actions {
    display: flex;
    gap: var(--space-sm);
    margin-top: var(--space-sm);
  }

  .picker-actions .btn {
    flex: 1;
    justify-content: center;
  }
</style>
