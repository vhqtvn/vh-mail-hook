import type { Load } from '@sveltejs/kit';
import { redirect } from '@sveltejs/kit';
import { auth } from '$lib/stores/auth';
import { get } from 'svelte/store';

export const load: Load = async () => {
  const user = get(auth);
  
  if (user) {
    throw redirect(302, '/mailboxes');
  }

  return {
    user,
  };
}; 