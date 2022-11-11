const defaultTheme = require('tailwindcss/defaultTheme');
const colors = require('tailwindcss/colors');
const plugin = require('tailwindcss/plugin');
const flattenColorPalette = require('tailwindcss/lib/util/flattenColorPalette').default;

module.exports = {
	content: ['./src/**/*.{ts,tsx}'],
	corePlugins: {
		// Conflicts with Docusaurus's styles
		preflight: false,
	},
	darkMode: 'class',
	plugins: [
		// Generate CSS variables so that we may overwrite Docusaurus styles with them
		plugin(({ addComponents, config }) => {
			const cssVars = {};

			Object.entries({
				colors: 'color',
				fontFamily: 'font-family',
				margin: 'margin',
				padding: 'padding',
			}).forEach(([key, name]) => {
				let setting = config(`theme.${key}`, []);

				if (key === 'colors') {
					setting = flattenColorPalette(setting);
				}

				Object.entries(setting).forEach(([k, v]) => {
					const varName = k.toLocaleLowerCase().replace('/', '-').replace('.', '_');

					cssVars[`--moon-${name}-${varName}`] = Array.isArray(v) ? v.join(', ') : v;
				});
			});

			addComponents({
				':root': cssVars,
			});
		}),
	],
	theme: {
		extend: {
			fontFamily: {
				sans: ['"Plus Jakarta Sans"', ...defaultTheme.fontFamily.sans],
			},
		},
		// These aren't entirely accessible but work for now
		colors: {
			current: 'currentColor',
			transparent: 'transparent',
			black: '#000',
			white: '#fff',
			// https://maketintsandshades.com/#4A2EC6
			blurple: {
				50: '#dbd5f4',
				100: '#b7abe8',
				200: '#9282dd',
				300: '#6e58d1',
				400: '#4a2ec6',
				500: '#4329b2',
				600: '#3b259e',
				700: '#34208b',
				800: '#251763',
				900: '#160e3b',
			},
			// https://maketintsandshades.com/#BDC9DB
			gray: {
				50: '#f2f4f8',
				100: '#e5e9f1',
				200: '#d7dfe9',
				300: '#cad4e2',
				400: '#bdc9db',
				500: '#aab5c5',
				600: '#97a1af',
				700: '#848d99',
				800: '#5f656e',
				900: '#393c42',
			},
			// https://maketintsandshades.com/#A5CD00
			green: {
				50: '#edf5cc',
				100: '#dbeb99',
				200: '#c9e166',
				300: '#b7d733',
				400: '#a5cd00',
				500: '#95b900',
				600: '#84a400',
				700: '#739000',
				800: '#637b00',
				900: '#425200',
			},
			// https://maketintsandshades.com/#A879FF
			lavendar: {
				50: '#eee4ff',
				100: '#dcc9ff',
				200: '#cbafff',
				300: '#b994ff',
				400: '#a879ff',
				500: '#976de6',
				600: '#8661cc',
				700: '#7655b3',
				800: '#543d80',
				900: '#32244c',
			},
			// https://maketintsandshades.com/#FF9B24
			orange: {
				50: '#ffebd3',
				100: '#ffd7a7',
				200: '#ffc37c',
				300: '#ffaf50',
				400: '#ff9b24',
				500: '#e68c20',
				600: '#cc7c1d',
				700: '#b36d19',
				800: '#804e12',
				900: '#4c2e0b',
			},
			// https://maketintsandshades.com/#FF79FF
			pink: {
				50: '#ffd7ff',
				100: '#ffbcff',
				200: '#ffafff',
				300: '#ff94ff',
				400: '#ff79ff',
				500: '#e66de6',
				600: '#cc61cc',
				700: '#b355b3',
				800: '#803d80',
				900: '#4c244c',
			},
			// https://maketintsandshades.com/#6F53F3
			purple: {
				50: '#f1eefe',
				100: '#d4cbfb',
				200: '#c5bafa',
				300: '#b7a9f9',
				400: '#9a87f7',
				500: '#8c75f5',
				600: '#6f53f3',
				700: '#5942c2',
				800: '#433292',
				900: '#2c2161',
			},
			// https://maketintsandshades.com/#FF5B6B
			red: {
				50: '#ffced3',
				100: '#ffadb5',
				200: '#ff9da6',
				300: '#ff7c89',
				400: '#ff5b6b',
				500: '#e65260',
				600: '#cc4956',
				700: '#b3404b',
				800: '#802e36',
				900: '#4c1b20',
			},
			// https://maketintsandshades.com/#012A4A
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
			// https://maketintsandshades.com/#79D5E9
			teal: {
				50: '#e4f7fb',
				100: '#c9eef6',
				200: '#afe6f2',
				300: '#94dded',
				400: '#79d5e9',
				500: '#6dc0d2',
				600: '#61aaba',
				700: '#5595a3',
				800: '#49808c',
				900: '#30555d',
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
