const plugin = require('tailwindcss/plugin');
const flattenColorPalette = require('tailwindcss/lib/util/flattenColorPalette').default;

// 1 = 8px, assuming root font size is 16px
const SPACING = [0, 0.25, 0.5, 1, 1.5, 2, 2.5, 3, 4, 5, 6, 7, 8, 9, 10];

module.exports = {
	content: ['./src/**/*.{ts,tsx}'],
	corePlugins: {
		// Conflicts with Docusaurus's styles
		preflight: false,
	},
	plugins: [
		// Generate CSS variables so that we may overwrite Docusaurus styles with them
		plugin(({ addComponents, config }) => {
			const cssVars = {};

			Object.entries({
				colors: 'color',
				margin: 'margin',
				padding: 'padding',
			}).forEach(([key, name]) => {
				let setting = config(`theme.${key}`, []);

				if (key === 'colors') {
					setting = flattenColorPalette(setting);
				}

				Object.entries(setting).forEach(([k, v]) => {
					const varName = k.toLocaleLowerCase().replace('/', '-').replace('.', '_');

					cssVars[`--moon-${name}-${varName}`] = v;
				});
			});

			addComponents({
				':root': cssVars,
			});
		}),
	],
	theme: {
		spacing: Object.fromEntries(
			SPACING.map((spacing) => [spacing, spacing === 0 ? '0rem' : `${spacing / 2}rem`]),
		),
	},
};
