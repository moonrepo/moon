const workspacePreset = require('../../tailwind.config');

/** @type {import('tailwindcss').Config} */
module.exports = {
	presets: [workspacePreset],
	content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
};
