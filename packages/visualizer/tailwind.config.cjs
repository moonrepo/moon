const workspacePreset = require('../../tailwind.config');

/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
	presets: [workspacePreset],
};
