import { writable } from 'svelte/store';
import { get } from '$lib/api';

export interface User {
  id: string;
  email: string;
  created_at: string;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<User | null>(null);

  return {
    subscribe,
    setUser: (user: User | null) => set(user),
    logout: () => set(null),
    async checkAuth() {
      try {
        const response = await get('/api/auth/me');
        if (!response.ok) {
          set(null);
          return;
        }
        const user = await response.json();
        set(user);
      } catch (e: any) {
        set(null);
      }
    },
  };
}

export const auth = createAuthStore(); 