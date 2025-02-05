import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { resolve } from 'path';
import fs from 'fs';

export default defineConfig({
	plugins: [
		sveltekit(),
		{
			name: 'copy-swagger-html',
			generateBundle() {
				this.emitFile({
					type: 'asset',
					fileName: 'swagger.html',
					source: fs.readFileSync(
						resolve(__dirname, 'static/swagger.html'),
						'utf-8'
					)
				});
			}
		}
	],
	optimizeDeps: {
		include: ['swagger-ui-dist']
	},
	build: {
		// Build to a directory that our Rust code can easily include
		outDir: '../static',
		emptyOutDir: true,
		commonjsOptions: {
			include: [/swagger-ui-dist/]
		}
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
