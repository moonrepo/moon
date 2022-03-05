module.exports = {
	root: true,
	extends: ['beemo', 'beemo/node'],
	parserOptions: {
		project: 'tsconfig.eslint.json',
		tsconfigRootDir: __dirname,
	},
	overrides: [
		{
			files: ['scripts/**/*'],
			rules: {
				'no-console': 'off',
				'no-magic-numbers': 'off',
				'promise/prefer-await-to-callbacks': 'off',
			},
		},
	],
};
