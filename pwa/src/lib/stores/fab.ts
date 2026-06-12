import { writable } from 'svelte/store';

/** Incremented each time the FAB is pressed on the Projects page. */
export const projectFabTrigger = writable(0);

export function triggerProjectFab() {
  projectFabTrigger.update(n => n + 1);
}
