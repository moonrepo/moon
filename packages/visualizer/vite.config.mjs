/* eslint-disable import/no-extraneous-dependencies */

import preact from '@preact/preset-vite';
import cssInjectedByJsPlugin from 'vite-plugin-css-injected-by-js';
import { defineConfig } from 'vite-plus';

// https://vitejs.dev/config/
// oxlint-disable-next-line import/no-default-export
export default defineConfig({
	build: {
		outDir: 'dist',
		rolldownOptions: {
			output: {
				assetFileNames: `assets/[name].[ext]`,
				chunkFileNames: `assets/[name].js`,
				entryFileNames: `assets/[name].js`,
				manualChunks: undefined,
			},
		},
	},
	plugins: [preact(), cssInjectedByJsPlugin()],
});
