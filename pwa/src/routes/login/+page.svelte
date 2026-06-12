<script lang="ts">
  import { signIn, signUp } from '$lib/stores/auth';

  let email = '';
  let password = '';
  let error = '';
  let loading = false;
  let isSignUp = false;

  async function handleSubmit() {
    error = '';
    loading = true;
    try {
      if (isSignUp) {
        await signUp(email, password);
      } else {
        await signIn(email, password);
      }
    } catch (e: any) {
      error = e.message || 'Authentication failed';
    } finally {
      loading = false;
    }
  }

  function toggleMode() {
    isSignUp = !isSignUp;
    error = '';
  }
</script>

<div class="auth-page">
  <div class="auth-card">
    <h1 class="auth-title">TodoRS</h1>
    <p class="auth-subtitle">{isSignUp ? 'Create an account' : 'Sign in to your account'}</p>

    <form class="auth-form" on:submit|preventDefault={handleSubmit}>
      <input
        type="email"
        class="input"
        bind:value={email}
        placeholder="Email"
        aria-label="Email address"
        required
        disabled={loading}
      />
      <input
        type="password"
        class="input"
        bind:value={password}
        placeholder="Password"
        aria-label="Password"
        required
        minlength={6}
        disabled={loading}
      />

      {#if error}
        <div class="auth-error">{error}</div>
      {/if}

      <button type="submit" class="btn btn-primary" style="width: 100%; justify-content: center;" disabled={loading}>
        {loading ? 'Please wait...' : isSignUp ? 'Create Account' : 'Sign In'}
      </button>

      <button type="button" class="btn btn-tertiary" style="width: 100%; justify-content: center;" on:click={toggleMode} disabled={loading}>
        {isSignUp ? 'Already have an account? Sign in' : "Don't have an account? Sign up"}
      </button>
    </form>
  </div>
</div>

<style>
  .auth-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100dvh;
    padding: var(--space-lg);
    background-color: var(--color-canvas);
  }

  .auth-card {
    width: 100%;
    max-width: 400px;
    background-color: var(--color-surface-1);
    border: 1px solid var(--color-hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-xl);
  }

  .auth-title {
    font-family: var(--font-display);
    font-size: var(--text-headline);
    font-weight: var(--weight-headline);
    letter-spacing: var(--tracking-headline);
    color: var(--color-ink);
    text-align: center;
    margin-bottom: var(--space-xs);
  }

  .auth-subtitle {
    font-size: var(--text-body-sm);
    color: var(--color-ink-subtle);
    text-align: center;
    margin-bottom: var(--space-lg);
  }

  .auth-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .auth-error {
    background-color: var(--color-error-soft);
    color: var(--color-error);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    font-size: var(--text-caption);
  }
</style>
