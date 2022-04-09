const colors = require('tailwindcss/colors');
const plugin = require('tailwindcss/plugin');
const flattenColorPalette = require('tailwindcss/lib/util/flattenColorPalette').default;

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
		// These arent entirely accessible but work for now
		colors: {
			current: 'currentColor',
			transparent: 'transparent',
			black: '#000',
			white: '#fff',
			// https://maketintsandshades.com/#292940
			gray: {
				50: '#ffffff',
				100: '#eaeaec',
				200: '#d4d4d9',
				300: '#bfbfc6',
				400: '#a9a9b3',
				500: '#9494a0',
				600: '#7f7f8c',
				700: '#696979',
				800: '#545466',
				900: '#3e3e53',
			},
			// https://maketintsandshades.com/#89ff6e
			green: {
				50: '#e7ffe2',
				100: '#d0ffc5',
				200: '#b8ffa8',
				300: '#a1ff8b',
				400: '#89ff6e',
				500: '#7be663',
				600: '#60b34d',
				700: '#529942',
				800: '#37662c',
				900: '#1b3316',
			},
			// https://maketintsandshades.com/#fd77fd
			pink: {
				50: '#ffe4ff',
				100: '#fec9fe',
				200: '#feadfe',
				300: '#fd92fd',
				400: '#fd77fd',
				500: '#e46be4',
				600: '#ca5fca',
				700: '#984798',
				800: '#653065',
				900: '#331833',
			},
			// base #664ae8
			purple: {
				50: '#FBF9FF',
				100: '#F1EBFD',
				200: '#DDCDFA',
				300: '#C8B1F7',
				400: '#B194F4',
				500: '#9676EF',
				600: '#7758EA',
				700: '#5F45D4',
				800: '#4D37A4',
				900: '#3E2D7E',
			},
			red: colors.rose,
			// https://maketintsandshades.com/#012a4a
			slate: {
				50: '#99aab7',
				100: '#677f92',
				200: '#34556e',
				300: '#1a3f5c',
				400: '#012a4a',
				500: '#01223b',
				600: '#01192c',
				700: '#011525',
				800: '#00111e',
				900: '#00080f',
			},
			// https://maketintsandshades.com/#69e2e5
			teal: {
				50: '#e1f9fa',
				100: '#c3f3f5',
				200: '#a5eeef',
				300: '#87e8ea',
				400: '#69e2e5',
				500: '#5fcbce',
				600: '#54b5b7',
				700: '#4a9ea0',
				800: '#3f8889',
				900: '#357173',
			},
			yellow: colors.yellow,
		},
		// 1 = 8px, assuming root font size is 16px
		spacing: Object.fromEntries(
			[0, 0.25, 0.5, 1, 1.5, 2, 2.5, 3, 4, 5, 6, 7, 8, 9, 10].map((spacing) => [
				spacing,
				spacing === 0 ? '0rem' : `${spacing / 2}rem`,
			]),
		),
	},
};
