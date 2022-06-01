/* eslint-disable sort-keys */
// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
	docs: [
		'intro',
		{
			type: 'category',
			label: 'Getting started',
			items: ['install', 'setup-workspace', 'create-project', 'create-task', 'run-task'],
		},
		{
			type: 'category',
			label: 'Guides',
			items: [
				'guides/ci',
				'guides/open-source',
				{
					type: 'category',
					label: 'Examples',
					collapsed: false,
					collapsible: true,
					items: [
						'guides/examples/eslint',
						'guides/examples/jest',
						'guides/examples/packemon',
						'guides/examples/prettier',
						'guides/examples/typescript',
					],
					link: {
						type: 'generated-index',
						title: 'Examples',
						slug: '/guides/examples',
						keywords: ['guides', 'examples', 'tools'],
					},
				},
			],
			link: {
				type: 'generated-index',
				title: 'Guides',
				slug: '/guides',
				keywords: ['guides'],
			},
		},
		{
			type: 'category',
			label: 'Concepts',
			items: [
				'concepts/cache',
				'concepts/file-group',
				'concepts/file-pattern',
				'concepts/project',
				'concepts/target',
				'concepts/task',
				'concepts/token',
				'concepts/toolchain',
				'concepts/workspace',
			],
			link: {
				type: 'generated-index',
				title: 'Concepts',
				slug: '/concepts',
				keywords: ['concepts'],
			},
		},
		{
			type: 'category',
			label: 'Config files',
			items: ['config/workspace', 'config/global-project', 'config/project'],
			link: {
				type: 'generated-index',
				title: 'Config files',
				slug: '/config',
				keywords: ['config'],
			},
		},
		{
			type: 'category',
			label: 'Commands',
			items: [
				'commands/overview',
				'commands/bin',
				'commands/ci',
				'commands/init',
				'commands/project',
				'commands/project-graph',
				'commands/run',
				'commands/setup',
				'commands/teardown',
			],
			link: {
				type: 'generated-index',
				title: 'Commands',
				slug: '/commands',
				keywords: ['cli', 'commands'],
			},
		},
		'terminology',
	],
};

// eslint-disable-next-line import/no-commonjs
module.exports = sidebars;
