import type { Load } from '@sveltejs/kit';
import { auth } from '$lib/stores/auth';
import { get } from '$lib/api';

export const load: Load = async ({ fetch }) => {
  // Check authentication status
  try {
    const response = await get('/api/auth/me', { requireAuth: true });
    if (response.ok) {
      const user = await response.json();
      auth.setUser(user);
      return {
        user,
      };
    }
  } catch (e: any) {
    if (typeof window !== 'undefined') {
      window.localStorage.removeItem('auth_token');
    }
  }

  return {
    user: null,
  };
}; 