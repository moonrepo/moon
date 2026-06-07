import globals from 'globals';
import { defineConfig } from 'vite-plus';

function mapGlobals(
	vars: Record<string, boolean>,
): Record<string, 'readonly' | 'writable' | 'off'> {
	return Object.fromEntries(
		Object.entries(vars).map(([key, value]) => [key, value ? 'readonly' : 'off']),
	);
}

const ignorePatterns = [
	'.moon/cache/',
	'.moon/docker/',
	'**/.docusaurus/',
	'**/coverage/',
	'**/build/',
	'**/cjs/',
	'**/dist/',
	'**/dts/',
	'**/esm/',
	'**/lib/',
	'**/mjs/',
	'**/node_modules/',
	'**/umd/',
	'**/__fixtures__/',
	'**/*.d.ts',
	'crates/config/templates/',
	'packages/visualizer/*timestamp*',
	'target/',
	'wasm/target/',
];

export default defineConfig({
	lint: {
		plugins: ['oxc', 'typescript', 'unicorn', 'react', 'promise', 'import', 'node'],
		jsPlugins: [
			{
				name: 'vite-plus',
				specifier: 'vite-plus/oxlint-plugin',
			},
		],
		categories: {
			correctness: 'warn',
		},
		options: {
			typeAware: true,
			typeCheck: true,
		},
		env: {
			builtin: true,
			commonjs: true,
		},
		globals: mapGlobals(globals.node),
		ignorePatterns: [
			...ignorePatterns,
			'**/*.json',
			'**/*.snap',
			'**/*.mdx',
			'**/*.css',
			'**/*.html',
			'scenarios/',
		],
	},
	fmt: {
		singleQuote: true,
		sortImports: true,
		sortPackageJson: false,
		ignorePatterns: [...ignorePatterns, '**/*.toml'],
		overrides: [
			{
				files: ['**/*.{js,cjs,mjs,ts,tsx,cts,mts,html,css}'],
				options: {
					useTabs: true,
				},
			},
			{
				files: ['**/*.md'],
				options: {
					proseWrap: 'always',
				},
			},
		],
	},
	test: {
		include: ['packages/**/*.test.ts'],
		environment: 'node',
		globals: false,
	},
	pack: {
		dts: {
			build: true,
		},
		exports: {
			legacy: true,
		},
		format: ['cjs'],
		outDir: 'cjs',
		platform: 'node',
		sourcemap: true,
	},
});
