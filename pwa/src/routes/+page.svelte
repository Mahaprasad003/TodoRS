<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { authStore } from '$lib/stores/auth';

  onMount(() => {
    if (!$authStore.loading) {
      if ($authStore.user) {
        goto('/inbox');
      } else {
        goto('/login');
      }
    }
  });
</script>

<div class="loading-screen">
  <div class="loading-spinner"></div>
  <div class="loading-text">Redirecting...</div>
</div>

<style>
  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100dvh;
    gap: var(--space-md);
    background-color: var(--color-canvas);
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-hairline);
    border-top-color: var(--color-primary);
    border-radius: 9999px;
    animation: spin 0.8s linear infinite;
  }

  .loading-text {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
  }
</style>
