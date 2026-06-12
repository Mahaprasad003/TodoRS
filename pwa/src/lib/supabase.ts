import { createClient, type Session, type User } from '@supabase/supabase-js';
import { writable } from 'svelte/store';

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY;

export const supabase = createClient(supabaseUrl, supabaseAnonKey, {
  auth: {
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: true,
  },
});

// ── Auth store backed by Supabase session ──

export const authStore = writable<{
  user: User | null;
  session: Session | null;
  loading: boolean;
}>({
  user: null,
  session: null,
  loading: true,
});

// ── Initialize auth listener ──

export function initAuth() {
  // Get initial session
  supabase.auth.getSession().then(({ data: { session } }) => {
    authStore.set({
      user: session?.user ?? null,
      session,
      loading: false,
    });
  });

  // Listen for auth changes
  supabase.auth.onAuthStateChange((event, session) => {
    authStore.set({
      user: session?.user ?? null,
      session,
      loading: false,
    });
  });
}

// ── Auth methods ──

export async function signIn(email: string, password: string) {
  const { data, error } = await supabase.auth.signInWithPassword({ email, password });
  if (error) throw error;
  return data;
}

export async function signUp(email: string, password: string) {
  const { data, error } = await supabase.auth.signUp({ email, password });
  if (error) throw error;
  return data;
}

export async function signOut() {
  const { error } = await supabase.auth.signOut();
  if (error) throw error;
}
