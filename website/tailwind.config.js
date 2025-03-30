// @ts-expect-error Ignore
const workspacePreset = require('../tailwind.config');

/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ['./src/**/*.{ts,tsx}', './docs/**/*.mdx'],
	corePlugins: {
		// Conflicts with Docusaurus's styles
		preflight: false,
	},
	// using it as a preset causes an error with prism, so we unpack it
	...workspacePreset,
};
