// eslint-disable-next-line import/no-commonjs
module.exports = {
	ignorePatterns: ['prism.config.js', 'tailwind.config.js'],
	rules: {
		// This fails on windows for some reason
		'import/named': 'off',

		// Docusaurus requires default exports for components
		'import/no-default-export': 'off',

		// Tailwind composition
		'no-magic-numbers': 'off',
	},
};
