/* eslint-disable sort-keys */
// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
	docs: [
		'intro',
		{
			type: 'category',
			label: 'Getting started',
			items: ['install', 'setup-workspace'],
		},
		{
			type: 'category',
			label: 'Concepts',
			items: [
				'concepts/workspace',
				'concepts/toolchain',
				'concepts/project',
				'concepts/task',
				'concepts/cache',
			],
		},
		{
			type: 'category',
			label: 'Config files',
			items: ['config/workspace', 'config/global-project', 'config/project'],
		},
		{
			type: 'category',
			label: 'Commands',
			items: [
				'commands/bin',
				'commands/ci',
				'commands/init',
				'commands/project',
				'commands/project-graph',
				'commands/setup',
				'commands/teardown',
			],
		},
	],
};

// eslint-disable-next-line import/no-commonjs
module.exports = sidebars;
