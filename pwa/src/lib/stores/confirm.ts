import { writable } from 'svelte/store';

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

interface ConfirmState {
  show: boolean;
  options: ConfirmOptions;
  resolve: ((value: boolean) => void) | null;
}

export const confirmStore = writable<ConfirmState>({
  show: false,
  options: { title: '', message: '' },
  resolve: null,
});

/** Show a native-style confirm dialog. Returns true if confirmed, false if cancelled. */
export function confirm(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    confirmStore.set({
      show: true,
      options,
      resolve,
    });
  });
}

/** Called by the ConfirmDialog when user taps Confirm or Cancel. */
export function confirmResponse(value: boolean) {
  confirmStore.update((state) => {
    if (state.resolve) state.resolve(value);
    return { show: false, options: { title: '', message: '' }, resolve: null };
  });
}
