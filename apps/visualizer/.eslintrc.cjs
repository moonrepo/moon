module.exports = {
	env: { browser: true },
	extends: ['eslint:recommended', 'plugin:react/recommended'],
	rules: {
		'node/no-unpublished-import': 'off',
		'@typescript-eslint/no-unsafe-argument': 'off',
		'@typescript-eslint/no-unsafe-assignment': 'off',
		'@typescript-eslint/triple-slash-reference': 'off',
		'@typescript-eslint/no-unsafe-member-access': 'off',
	},
	settings: {
		react: { version: 'detect' },
	},
};
