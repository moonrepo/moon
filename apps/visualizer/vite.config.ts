import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { createGitignore } from './plugins/createGitignore';

// https://vitejs.dev/config/
/* eslint-disable import/no-default-export */
export default defineConfig({
	plugins: [react(), createGitignore()],
});
