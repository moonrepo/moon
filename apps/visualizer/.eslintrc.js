module.exports = {
	extends: ['eslint:recommended', 'plugin:react/recommended'],
	parserOptions: {
		extraFileExtensions: ['.tsx'],
		parser: '@typescript-eslint/parser',
		project: 'tsconfig.vitest.json',
		tsconfigRootDir: __dirname,
	},
	rules: {
		'node/no-unpublished-import': 'off',
		// Imported Vue files resolve as any
		'@typescript-eslint/no-unsafe-argument': 'off',
		'@typescript-eslint/no-unsafe-assignment': 'off',
	},
};
