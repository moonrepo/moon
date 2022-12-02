/* eslint-disable node/no-unpublished-import */
/* eslint-disable import/no-extraneous-dependencies */

import { defineConfig } from 'vite';
import cssInjectedByJsPlugin from 'vite-plugin-css-injected-by-js';
import preact from '@preact/preset-vite';

// https://vitejs.dev/config/
// eslint-disable-next-line import/no-default-export
export default defineConfig({
	build: {
		outDir: 'dist',
		rollupOptions: {
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
