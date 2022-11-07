import { writeFileSync } from 'fs';
import type { PluginOption } from 'vite';
import { normalizePath } from 'vite';

/**
 * The `dist` directory is deleted every time the bundle is rebuilt. The `.gitignore` file
 * inside it needs to be generated again for the folder to remain committed to git. Hence
 * we use vite's hook mechanism to manually re-create one.
 */
export function createGitignore(): PluginOption {
	return {
		apply: 'build',
		closeBundle: () => {
			writeFileSync(
				normalizePath('dist/.gitignore'),
				`
*
!.gitignore
`.trim(),
			);
		},
		name: 'create-gitignore',
	};
}
