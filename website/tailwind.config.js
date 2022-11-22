const workspacePreset = require('../tailwind.config');

/** @type {import('tailwindcss').Config} */
module.exports = {
	corePlugins: {
		// Conflicts with Docusaurus's styles
		preflight: false,
	},
	content: ['./src/**/*.{ts,tsx}'],
	// using it as a preset causes an error with prism, so we unpack it
	...workspacePreset,
};
