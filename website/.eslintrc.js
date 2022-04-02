// eslint-disable-next-line import/no-commonjs
module.exports = {
	ignorePatterns: ['tailwind.config.js'],
	rules: {
		// Docusaurus requires default exports for components
		'import/no-default-export': 'off',

		'no-magic-numbers': 'off',
	},
};
