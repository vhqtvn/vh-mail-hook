import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		// Build to a directory that our Rust code can easily include
		outDir: '../static',
		emptyOutDir: true,
	}
});
