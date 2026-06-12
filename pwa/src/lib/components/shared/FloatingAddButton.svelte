<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  function handleClick() {
    dispatch('open');
  }
</script>

<button
  class="fab"
  on:click={handleClick}
  aria-label="Add task"
>
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
    <path d="M12 5V19M5 12H19" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>
  </svg>
</button>

<style>
  .fab {
    position: fixed;
    bottom: calc(64px + 16px);
    right: 24px;
    width: 56px;
    height: 56px;
    border-radius: 9999px;
    background-color: var(--color-primary);
    color: var(--color-on-primary);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 16px rgba(94, 106, 210, 0.4);
    z-index: 90;
    transition: transform 0.15s ease, box-shadow 0.15s ease;
    padding: 0;
  }

  .fab:hover {
    transform: scale(1.08);
    box-shadow: 0 6px 20px rgba(94, 106, 210, 0.5);
  }

  .fab:active {
    transform: scale(0.95);
  }

  @media (min-width: 768px) {
    .fab {
      display: none;
    }
  }
</style>
