import { writable, get as getStore } from 'svelte/store';
import { get, post } from '$lib/api';
import { goto } from '$app/navigation';

export interface User {
  id: string;
  email: string;
  created_at: string;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<User | null>(null);

  return {
    subscribe,
    setUser: (user: User | null) => {
      set(user);
    },
    logout: async () => {
      localStorage.removeItem('auth_token');
      set(null);
      await goto('/');
    },
    async login(token: string, user: User) {
      localStorage.setItem('auth_token', token);
      set(user);
      await goto('/mailboxes');
    },
    async register(token: string, user: User) {
      localStorage.setItem('auth_token', token);
      set(user);
      await goto('/mailboxes');
    },
    async checkAuth() {
      try {
        const response = await get<User>('/api/auth/me');
        if (!response.success || !response.data) {
          set(null);
          return null;
        }
        set(response.data);
        return response.data;
      } catch (e: any) {
        localStorage.removeItem('auth_token');
        set(null);
        return null;
      }
    },
    isAuthenticated() {
      return !!getStore(this);
    }
  };
}

export const auth = createAuthStore(); 