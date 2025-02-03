<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';

	let theme: string = 'light';

	onMount(() => {
		// Initialize theme from localStorage or default to light
		const savedTheme = localStorage.getItem('theme') || 'light';
		theme = savedTheme;
		document.documentElement.setAttribute('data-theme', savedTheme);
		// Check auth status on mount
		auth.checkAuth();
	});

	function toggleTheme() {
		theme = theme === 'light' ? 'dark' : 'light';
		localStorage.setItem('theme', theme);
		document.documentElement.setAttribute('data-theme', theme);
	}

	async function handleLogout() {
		await auth.logout();
	}
</script>

<div class="min-h-screen bg-base-100">
	<div class="navbar h-14 px-8 sm:px-12 bg-base-100 border-b border-base-200 shadow-sm">
		<div class="flex-1">
			<a href="/" class="flex items-center pl-2">
				<div class="p-1.5 rounded-xl bg-primary/10">
					<svg xmlns="http://www.w3.org/2000/svg" class="h-7 w-7 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
					</svg>
				</div>
				<span class="ml-3 text-xl font-semibold tracking-wide text-base-content">Mail Hook</span>
			</a>
		</div>
		<div class="flex items-center gap-6">
			{#if !$auth}
				<div class="flex items-center gap-4">
					<a href="/auth/login" class="text-sm font-medium text-base-content/70 hover:text-primary transition-colors">Sign in</a>
					<a href="/auth/register" class="inline-flex items-center h-8 px-3.5 rounded-lg bg-primary text-primary-content text-sm font-medium hover:bg-primary-focus transition-colors">Sign up</a>
				</div>
			{:else}
				<a 
					href="/mailboxes" 
					class="inline-flex items-center gap-2 h-8 px-3 rounded-lg bg-primary text-primary-content text-sm font-medium hover:bg-primary-focus transition-all"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
					</svg>
					<span>Mailboxes</span>
				</a>
				<div class="dropdown dropdown-end">
					<div class="tooltip tooltip-bottom" data-tip="Account Settings">
						<label tabindex="-1" class="avatar flex items-center gap-1 cursor-pointer group hover:opacity-90 transition-opacity">
							<div class="w-8 h-8 rounded-full ring-2 ring-primary/20 group-hover:ring-primary/40 transition-all">
								<img src={`https://ui-avatars.com/api/?name=${$auth.username}&background=random`} alt="User avatar" />
							</div>
							<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 text-base-content/50 group-hover:text-base-content/70 transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
							</svg>
						</label>
					</div>
					<ul tabindex="-1" class="mt-2 z-[1] p-2 shadow-lg menu menu-sm dropdown-content rounded-lg w-52 bg-base-100 border border-base-200">
						<li><a href="/settings" class="py-2 px-4 text-base-content hover:bg-base-200 rounded-lg">Settings</a></li>
						<div class="divider my-1 opacity-10"></div>
						<li><button on:click={handleLogout} class="py-2 px-4 text-error hover:bg-error/10 rounded-lg">Logout</button></li>
					</ul>
				</div>
			{/if}
			<div class="tooltip tooltip-bottom" data-tip={theme === 'light' ? 'Switch to Dark Mode' : 'Switch to Light Mode'}>
				<button 
					class="inline-flex items-center justify-center h-8 w-8 rounded-lg bg-base-200 hover:bg-base-300 transition-all text-base-content/70 hover:text-base-content"
					on:click={toggleTheme} 
					aria-label="Toggle theme"
				>
					{#if theme === 'light'}
						<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
						</svg>
					{:else}
						<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707" />
						</svg>
					{/if}
				</button>
			</div>
		</div>
	</div>

	<main class="container mx-auto px-4 py-8">
		<slot />
	</main>
</div>
