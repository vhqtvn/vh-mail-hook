import type { Load } from '@sveltejs/kit';
import { redirect } from '@sveltejs/kit';

export const load: Load = async ({ parent }) => {
  const { user } = await parent();
  
  if (user) {
    throw redirect(302, '/mailboxes');
  }

  return {
    user,
  };
}; 