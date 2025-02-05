import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		// Build to a directory that our Rust code can easily include
		outDir: '../static',
		emptyOutDir: true,
	},
	server: {
		allowedHosts: true,
	},
	// Look for .env files in the project root
	envDir: '../../..',
	// Ensure Vite processes these env vars
	envPrefix: ['VITE_'],
	define: {
		// Explicitly define environment variables for production
		'import.meta.env.VITE_TELEGRAM_BOT_NAME': JSON.stringify(process.env.VITE_TELEGRAM_BOT_NAME),
	}
});
