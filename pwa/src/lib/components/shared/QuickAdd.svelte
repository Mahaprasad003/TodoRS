<script lang="ts">
  import { get } from 'svelte/store';
  import { addTask } from '$lib/stores/tasks';
  import { addProject, activeProjects } from '$lib/stores/projects';
  import { parse, resolveDatetime, resolveRecurrence } from '$lib/parser';

  /** If set, new tasks will be assigned to this project. */
  export let projectId: string | null = null;
  export let placeholder = 'Add a task... e.g. "buy milk tomorrow 3pm p1"';

  let value = '';
  let submitting = false;
  let toast: string | null = null;
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string) {
    toast = msg;
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => { toast = null; }, 2500);
  }

  async function handleSubmit() {
    const trimmed = value.trim();
    if (!trimmed || submitting) return;

    submitting = true;
    try {
      // Parse the input
      const parsed = parse(trimmed);
      const dueAt = resolveDatetime(parsed.dueDate, parsed.dueTime) || null;
      const rec = resolveRecurrence(parsed.recurrence, parsed.anchorMode, parsed.waitForCompletion);

      // Resolve project: match +name to existing projects, or create one
      let resolvedProjectId: string | null = null;
      if (parsed.project) {
        const allProjects = get(activeProjects);
        const existing = allProjects.find(p => p.name.toLowerCase() === parsed.project!.toLowerCase());
        if (existing) {
          resolvedProjectId = existing.id;
        } else {
          // Auto-create the project
          await addProject(parsed.project);
          // Find the newly created project by name from fresh store read
          const updatedProjects = get(activeProjects);
          const created = updatedProjects.find(p => p.name.toLowerCase() === parsed.project!.toLowerCase());
          if (created) resolvedProjectId = created.id;
        }
      }

      // Build toast preview
      const parts: string[] = [parsed.title];
      if (parsed.priority !== 'none') parts.push(`(${parsed.priority})`);
      if (dueAt) {
        const d = new Date(dueAt);
        const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        const date = d.toLocaleDateString([], { month: 'short', day: 'numeric' });
        parts.push(`${date} ${time}`);
      }
      if (parsed.project) parts.push(`+${parsed.project}`);
      if (parsed.tags.length) parts.push(`@${parsed.tags.join(' @')}`);
      if (rec) parts.push(`every ${rec.interval > 1 ? rec.interval + ' ' : ''}${rec.kind}`);

      await addTask(parsed.title, {
        // Use the prop projectId if set (from page context), otherwise use NLP-parsed project
        project_id: projectId || resolvedProjectId || undefined,
        priority: parsed.priority !== 'none' ? parsed.priority : undefined,
        due_at: dueAt || undefined,
        recurrence: rec?.kind || undefined,
      });

      showToast(`Added: ${parts.join(' — ')}`);
      value = '';
    } catch (e: any) {
      console.error('QuickAdd error:', e);
      showToast(`Failed: ${e.message || 'Unknown error'}`);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSubmit();
    }
  }
</script>

<div class="quick-add-wrapper">
  <div class="quick-add-bar">
    <svg class="quick-add-icon" width="18" height="18" viewBox="0 0 24 24" fill="none">
      <path d="M12 5V19M5 12H19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
    <input
      type="text"
      class="quick-add-input"
      bind:value={value}
      on:keydown={handleKeydown}
      {placeholder}
      disabled={submitting}
      aria-label="Quick add task with natural language"
    />
    <button class="btn btn-primary quick-add-btn" on:click={handleSubmit} disabled={!value.trim() || submitting}>
      {submitting ? 'Adding...' : 'Add'}
    </button>
  </div>

</div>

<!-- Floating toast overlay (fixed position, no layout shift) -->
{#if toast}
  <div class="quick-add-toast">
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
      <path d="M4 8L7 11L12 5" stroke="var(--color-success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    {toast}
  </div>
{/if}

<style>
  .quick-add-wrapper {
    margin-bottom: var(--space-lg);
  }

  .quick-add-bar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    background-color: var(--color-surface-1);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-xs) var(--space-sm);
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .quick-add-bar:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 1px var(--color-primary);
  }

  .quick-add-icon {
    color: var(--color-ink-subtle);
    flex-shrink: 0;
  }

  .quick-add-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: var(--text-body);
    color: var(--color-ink);
    font-family: var(--font-body);
    padding: var(--space-sm) 0;
  }

  .quick-add-input::placeholder {
    color: var(--color-ink-tertiary);
  }

  .quick-add-btn {
    flex-shrink: 0;
  }

  .quick-add-toast {
    position: fixed;
    bottom: calc(64px + var(--space-lg));
    left: 50%;
    transform: translateX(-50%);
    z-index: 500;
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-lg);
    font-size: var(--text-body-sm);
    color: var(--color-ink);
    background-color: var(--color-surface-2);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    animation: toastIn 0.2s ease;
    white-space: nowrap;
    max-width: 90vw;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @keyframes toastIn {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  @media (min-width: 768px) {
    .quick-add-toast {
      bottom: var(--space-lg);
    }
  }
</style>
