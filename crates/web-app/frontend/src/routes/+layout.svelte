<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { removeAuthToken } from '$lib/api';
	import { auth } from '$lib/stores/auth';

	let theme = 'light';

	onMount(() => {
		// Check system preference
		if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
			theme = 'dark';
		}
		document.documentElement.setAttribute('data-theme', theme);
	});

	function toggleTheme() {
		theme = theme === 'light' ? 'dark' : 'light';
		document.documentElement.setAttribute('data-theme', theme);
	}

	function handleLogout() {
		removeAuthToken();
		auth.logout();
		window.location.href = '/';
	}
</script>

<div class="min-h-screen bg-base-100">
	<div class="navbar bg-base-200">
		<div class="flex-1">
			<a href="/" class="btn btn-ghost text-xl">Mail Hook</a>
		</div>
		<div class="flex-none gap-2">
			<button class="btn btn-ghost btn-circle" on:click={toggleTheme} aria-label="Toggle theme">
				{#if theme === 'light'}
					<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
					</svg>
				{:else}
					<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707" />
					</svg>
				{/if}
			</button>
			{#if !$page.data.user}
				<a href="/auth/login" class="btn btn-primary">Sign In</a>
				<a href="/auth/register" class="btn">Sign Up</a>
			{:else}
				<div class="dropdown dropdown-end">
					<button class="btn btn-ghost btn-circle" aria-haspopup="true" aria-expanded="false">
						<div class="avatar">
							<div class="w-10 rounded-full">
								<img src={`https://ui-avatars.com/api/?name=${$page.data.user.email}`} alt="User avatar" />
							</div>
						</div>
					</button>
					<ul class="mt-3 z-[1] p-2 shadow menu menu-sm dropdown-content bg-base-200 rounded-box w-52" role="menu">
						<li role="none"><a href="/mailboxes" role="menuitem">Mailboxes</a></li>
						<li role="none"><a href="/settings" role="menuitem">Settings</a></li>
						<li role="none"><button on:click={handleLogout} role="menuitem" class="w-full text-left">Logout</button></li>
					</ul>
				</div>
			{/if}
		</div>
	</div>

	<main class="container mx-auto px-4 py-8">
		<slot />
	</main>
</div>
