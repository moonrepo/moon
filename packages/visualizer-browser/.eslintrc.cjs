module.exports = {
	env: { browser: true, es2021: true },
	extends: ['preact'],
	parser: '@typescript-eslint/parser',
	parserOptions: {
		ecmaFeatures: { jsx: true },
		ecmaVersion: 12,
		sourceType: 'module',
	},
};
