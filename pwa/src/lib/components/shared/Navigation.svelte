<script lang="ts">
  import { page } from '$app/stores';
  import { openSearch } from '$lib/stores/search';

  const navItems = [
    { href: '/inbox', label: 'Inbox', icon: 'inbox' },
    { href: '/today', label: 'Today', icon: 'today' },
    { href: '/upcoming', label: 'Upcoming', icon: 'upcoming' },
    { href: '/projects', label: 'Projects', icon: 'projects' },
    { href: '/completed', label: 'Done', icon: 'completed' },
  ];

  $: currentPath = $page.url.pathname;
</script>

<nav class="bottom-nav" aria-label="Main navigation">
  {#each navItems as item}
    <a
      href={item.href}
      class="nav-item"
      class:active={currentPath === item.href}
    >
      <div class="nav-item-icon">
        {#if item.icon === 'inbox'}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <path d="M3 12H7L10 8L14 14L17 10L21 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M3 4V20H21V4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if item.icon === 'today'}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <rect x="3" y="4" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2"/>
            <path d="M3 10H21" stroke="currentColor" stroke-width="2"/>
            <circle cx="12" cy="16" r="2" fill="currentColor"/>
          </svg>
        {:else if item.icon === 'upcoming'}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <rect x="3" y="4" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2"/>
            <path d="M3 10H21" stroke="currentColor" stroke-width="2"/>
            <path d="M8 2V6M16 2V6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M12 14V18M14 16H10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        {:else if item.icon === 'projects'}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <path d="M3 7V5C3 3.89543 3.89543 3 5 3H9L11 6H19C20.1046 6 21 6.89543 21 8V19C21 20.1046 20.1046 21 19 21H5C3.89543 21 3 20.1046 3 19V7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if item.icon === 'completed'}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <rect x="3" y="4" width="18" height="16" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M8 12L11 15L16 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {/if}
      </div>
      <span class="nav-item-label">{item.label}</span>
    </a>
  {/each}

  <!-- Search action button -->
  <button
    type="button"
    class="nav-item"
    on:click={openSearch}
    aria-label="Search tasks"
  >
    <div class="nav-item-icon">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
        <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2"/>
        <path d="M16 16L21 21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
    </div>
    <span class="nav-item-label">Search</span>
  </button>
</nav>

<style>
  .bottom-nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 64px;
    background-color: var(--color-canvas);
    border-top: 1px solid var(--color-hairline);
    display: flex;
    align-items: center;
    justify-content: space-around;
    z-index: 100;
    padding: 0 var(--space-md);
  }

  @media (min-width: 768px) {
    .bottom-nav {
      display: none;
    }
  }

  .nav-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-md);
    text-decoration: none;
    color: var(--color-ink-subtle);
    transition: color 0.15s ease;
  }

  .nav-item.active {
    color: var(--color-primary);
  }

  .nav-item:hover {
    color: var(--color-ink);
  }

  .nav-item-icon {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .nav-item-label {
    font-size: 10px;
    font-weight: 500;
  }
</style>
