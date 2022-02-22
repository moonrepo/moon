module.exports = {
	root: true,
	extends: ['beemo', 'beemo/node'],
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
