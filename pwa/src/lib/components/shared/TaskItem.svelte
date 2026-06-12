<script lang="ts">
  import { toggleTaskComplete, removeTask } from '$lib/stores/tasks';
  import type { TaskRecord } from '$lib/db/schema';
  import { getProject } from '$lib/db/projects';
  import { openEdit } from '$lib/stores/edit';
  import { confirm } from '$lib/stores/confirm';

  export let task: TaskRecord;

  let projectName = '';
  let showMeta = false;

  $: {
    if (task.project_id) {
      getProject(task.project_id).then(p => {
        projectName = p?.name || '';
      });
    }
  }

  async function handleToggle() {
    await toggleTaskComplete(task.id);
  }

  async function handleDelete() {
    const ok = await confirm({ title: 'Delete task', message: 'This task will be moved to the archive.', confirmLabel: 'Delete', danger: true });
    if (ok) {
      await removeTask(task.id);
    }
  }

  function formatDueDate(dateStr?: string): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    const today = new Date();
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);

    const datePart = dateStr.slice(0, 10);
    const todayPart = today.toISOString().slice(0, 10);
    const tomorrowPart = tomorrow.toISOString().slice(0, 10);
    const yesterdayPart = yesterday.toISOString().slice(0, 10);

    // Show time if it's not midnight (00:00:00)
    const hasTime = dateStr.includes('T') && !dateStr.endsWith('T00:00:00.000Z') && !dateStr.endsWith('T00:00:00Z');
    const time = hasTime ? date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '';

    if (datePart === todayPart) {
      return hasTime ? `Today ${time}` : 'Today';
    }
    if (datePart === tomorrowPart) return hasTime ? `Tomorrow ${time}` : 'Tomorrow';
    if (datePart === yesterdayPart) return hasTime ? `Yesterday ${time}` : 'Yesterday';
    const dateLabel = date.toLocaleDateString([], { month: 'short', day: 'numeric' });
    return hasTime ? `${dateLabel} ${time}` : dateLabel;
  }

  function chipClass(p: string): string {
    return p !== 'none' ? `priority-${p}` : '';
  }

  function handleEdit() {
    if (task.status === 'completed') return;
    openEdit(task);
  }

  // ── Swipe state ──
  let swiping = false;
  let swipeDelta = 0;
  let startX = 0;
  let startY = 0;
  let actionTriggered = false;

  const SWIPE_THRESHOLD = 80;

  $: swipeBackground = swiping && Math.abs(swipeDelta) > 20
    ? swipeDelta > 0
      ? 'var(--color-swipe-complete)'
      : 'var(--color-swipe-delete)'
    : '';

  function handleTouchStart(e: TouchEvent) {
    if (task.status === 'completed') return;
    const touch = e.touches[0];
    startX = touch.clientX;
    startY = touch.clientY;
    swiping = true;
    swipeDelta = 0;
    actionTriggered = false;
  }

  function handleTouchMove(e: TouchEvent) {
    if (!swiping || actionTriggered) return;
    const touch = e.touches[0];
    const deltaX = touch.clientX - startX;
    const deltaY = touch.clientY - startY;

    // Only swipe if mostly horizontal
    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaY) < 40) {
      swipeDelta = deltaX;
      // Prevent page scroll while swiping horizontally
      e.preventDefault();
    }
  }

  function handleTouchEnd() {
    if (!swiping || actionTriggered) return;
    swiping = false;

    if (Math.abs(swipeDelta) > SWIPE_THRESHOLD) {
      if (swipeDelta > 0) {
        // Right swipe → complete
        actionTriggered = true;
        navigator.vibrate?.(10);
        swipeDelta = 0;
        handleToggle();
      } else {
        // Left swipe → delete
        actionTriggered = true;
        navigator.vibrate?.(10);
        swipeDelta = 0;
        handleDelete();
      }
    } else {
      swipeDelta = 0;
    }
  }

  // Compute transform (clamped, sign-aware)
  $: clampedDelta = Math.min(Math.abs(swipeDelta), 200) * Math.sign(swipeDelta) || 0;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="task-item {chipClass(task.priority)}"
  class:completed={task.status === 'completed'}
  class:swiping
  on:touchstart={handleTouchStart}
  on:touchmove={handleTouchMove}
  on:touchend={handleTouchEnd}
  style="transform: translateX({clampedDelta}px); background: {swipeBackground}"
>
  <button class="task-checkbox" class:checked={task.status === 'completed'} on:click={handleToggle} aria-label={task.status === 'completed' ? 'Uncomplete task' : 'Complete task'}>
    {#if task.status === 'completed'}
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M2.5 6L5 8.5L9.5 3.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    {/if}
  </button>

  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="task-content" on:click={handleEdit} on:keydown={() => {}}>
    <div class="task-title" class:completed={task.status === 'completed'}>{task.title || '(untitled)'}</div>
    <div class="task-meta">
      {#if task.due_at}
        <span class="task-meta-item">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1"/>
            <path d="M6 3V6L8 7" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
          </svg>
          {formatDueDate(task.due_at)}
        </span>
      {/if}

      {#if projectName}
        <span class="task-meta-item">{projectName}</span>
      {/if}
    </div>
  </div>

  <button class="btn-icon" on:click={handleDelete} aria-label="Delete task">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M3 4H13M5 4V3C5 2.44772 5.44772 2 6 2H10C10.5523 2 11 2.44772 11 3V4M7 7V11M9 7V11M4 4L4.44721 13.3416C4.48224 14.2685 5.24508 15 6.17333 15H9.82667C10.7549 15 11.5178 14.2685 11.5528 13.3416L12 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </button>
</div>

<style>
  .task-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
    border-left: 3px solid transparent;
    position: relative;
    overflow: hidden;
    touch-action: pan-y;
    transition: background-color 0.15s ease, border-color 0.15s ease, transform 0.05s linear;
  }

  .task-item:hover {
    background-color: var(--color-surface-2);
  }

  .task-item.completed {
    opacity: 0.7;
  }

  .task-item.swiping {
    transition: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .task-item.priority-urgent {
    border-left-color: var(--color-error);
  }

  .task-item.priority-high {
    border-left-color: var(--color-warning);
  }

  .task-item.priority-medium {
    border-left-color: var(--color-primary);
  }

  .task-checkbox {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    margin-top: 2px;
    border-radius: var(--radius-xs);
    border: 2px solid var(--color-hairline-strong);
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    color: var(--color-canvas);
    padding: 0;
  }

  .task-checkbox.checked {
    background-color: var(--color-success);
    border-color: var(--color-success);
  }

  .task-checkbox:not(.checked):hover {
    border-color: var(--color-primary);
  }

  .task-content {
    flex: 1;
    min-width: 0;
  }

  .task-title {
    font-size: var(--text-body);
    color: var(--color-ink);
    line-height: var(--lh-body);
    word-break: break-word;
  }

  .task-title.completed {
    text-decoration: line-through;
    color: var(--color-ink-subtle);
  }

  .task-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    margin-top: var(--space-xxs);
  }

  .task-meta-item {
    font-size: var(--text-caption);
    color: var(--color-ink-subtle);
    display: flex;
    align-items: center;
    gap: 2px;
  }

  /* Hide delete button on mobile — swipe handles it */
  @media (max-width: 767px) {
    .task-item .btn-icon {
      display: none;
    }
  }
</style>
