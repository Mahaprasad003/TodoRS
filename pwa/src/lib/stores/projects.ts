import { writable, derived } from 'svelte/store';
import { getAllProjects, createProject as dbCreateProject, softDeleteProject as dbSoftDeleteProject } from '$lib/db/projects';
import type { ProjectRecord } from '$lib/db/schema';
import { generateOperation, sync, getCurrentUserId } from '$lib/sync/client';

import { loadTasks } from './tasks';

export const projectsStore = writable<ProjectRecord[]>([]);

export const activeProjects = derived(projectsStore, $projects =>
  $projects.filter(p => !p.archived_at)
);

export async function loadProjects(userId: string): Promise<void> {
  const all = await getAllProjects(userId);
  projectsStore.set(all);
}

export async function addProject(name: string, color?: string): Promise<void> {
  const now = new Date().toISOString();
  const userId = getCurrentUserId() || '';
  const projectId = crypto.randomUUID();
  const project: ProjectRecord = {
    id: projectId,
    user_id: userId,
    name,
    color: color || '#5e6ad2',
    sort_order: 0,
    created_at: now,
    updated_at: now,
  };

  await dbCreateProject(project);
  projectsStore.update(projects => [...projects, project]);

  await generateOperation('project', projectId, 'create', {
    name,
    color: color || '#5e6ad2',
    sort_order: 0,
    created_at: now,
    updated_at: now,
  });

  sync().then(() => {
    if (userId) loadProjects(userId);
  }).catch(e => console.error('sync error after addProject:', e));
}

export async function removeProject(projectId: string): Promise<void> {
  const userId = getCurrentUserId();
  if (!userId) return;

  // Soft delete in IndexedDB
  await dbSoftDeleteProject(projectId);

  // Remove from store
  projectsStore.update(projects =>
    projects.map(p =>
      p.id === projectId ? { ...p, archived_at: new Date().toISOString(), updated_at: new Date().toISOString() } : p
    )
  );

  // Generate operation for sync
  await generateOperation('project', projectId, 'delete', { deleted_at: new Date().toISOString() });

  sync().then(() => {
    if (userId) loadProjects(userId);
    loadTasks(userId);
  }).catch(e => console.error('sync error after removeProject:', e));
}
