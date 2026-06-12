// Auth store — re-exports the session-backed store from the supabase client
// This is the canonical auth store for all components.
export { authStore, initAuth, signIn, signUp, signOut } from '$lib/supabase';
export type { User } from '@supabase/supabase-js';
