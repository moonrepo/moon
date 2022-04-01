/* eslint-disable node/no-unsupported-features/es-builtins */

// 1 = 8px, assuming root font size is 16px
const SPACING = [0.25, 0.5, 1, 1.5, 2, 2.5, 3, 4, 5, 6, 7, 8, 9, 10];

module.exports = {
	content: ['./src/**/*.{ts,tsx}'],
	plugins: [],
	theme: {
		spacing: Object.fromEntries(SPACING.map((spacing) => [spacing, `${spacing / 2}rem`])),
	},
};
