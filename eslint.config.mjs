import { defineConfig, globalIgnores } from 'eslint/config';
import moonConfig from 'eslint-config-moon';
import moonNodeConfig from 'eslint-config-moon/node';

const config = defineConfig([
	globalIgnores([
		'**/.docusaurus/',
		'**/build/',
		'**/cjs/',
		'**/coverage/',
		'**/dist/',
		'**/esm/',
		'**/lib/',
		'**/mjs/',
		'**/node_modules/',
		'**/*.d.ts',
		'**/*.json',
		'**/*.snap',
		'**/*.mdx',
		'**/*.css',
		'**/*.html',

		// It's buggy!
		'packages/visualizer/**/*',
	]),
	...moonConfig,
	...moonNodeConfig,
	{
		files: ['packages/types/**/*'],
		rules: {
			'unicorn/no-abusive-eslint-disable': 'off',
		},
	},
	{
		files: ['scripts/**/*'],
		rules: {
			'no-console': 'off',
			'no-magic-numbers': 'off',
			'no-process-exit': 'off',
			'import/no-extraneous-dependencies': 'off',
			'node/no-unpublished-import': 'off',
			'promise/prefer-await-to-callbacks': 'off',
			'unicorn/no-process-exit': 'off',
			'@typescript-eslint/no-unsafe-argument': 'off',
			'@typescript-eslint/no-unsafe-assignment': 'off',
			'@typescript-eslint/no-unsafe-call': 'off',
			'@typescript-eslint/no-unsafe-member-access': 'off',
			'@typescript-eslint/no-unsafe-return': 'off',
		},
	},
	{
		files: ['website/**/*'],
		rules: {
			'no-magic-numbers': 'off',
			'import/no-default-export': 'off',
			'import/no-unresolved': 'off',
			'node/no-unsupported-features/node-builtins': 'off',
		},
	},
]);

export default config;
