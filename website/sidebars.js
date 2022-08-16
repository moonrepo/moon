/* eslint-disable sort-keys */
// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
	docs: [
		'intro',
		{
			type: 'category',
			label: 'Getting started',
			items: [
				'install',
				'setup-workspace',
				'create-project',
				'create-task',
				'run-task',
				'migrate-to-moon',
			],
		},
		{
			type: 'category',
			label: 'Guides',
			items: [
				'guides/ci',
				'guides/open-source',
				'guides/profile',
				'guides/root-project',
				'guides/sharing-config',
				{
					type: 'category',
					label: 'Examples',
					collapsed: false,
					collapsible: true,
					items: [
						'guides/examples/eslint',
						'guides/examples/jest',
						'guides/examples/next',
						'guides/examples/packemon',
						'guides/examples/prettier',
						'guides/examples/react',
						'guides/examples/remix',
						'guides/examples/typescript',
						'guides/examples/vite',
						'guides/examples/vue',
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
				'commands/clean',
				'commands/dep-graph',
				'commands/init',
				{
					type: 'category',
					label: 'migrate',
					items: ['commands/migrate/from-package-json'],
					link: {
						type: 'generated-index',
						title: 'migrate',
						description: 'Operations for migrating existing projects to moon.',
						slug: '/commands/migrate',
						keywords: ['cli', 'commands', 'migrate'],
					},
				},
				'commands/project',
				'commands/project-graph',
				{
					type: 'category',
					label: 'query',
					items: ['commands/query/projects', 'commands/query/touched-files'],
					link: {
						type: 'generated-index',
						title: 'query',
						description:
							'Query information about moon, its projects, their tasks, the environment, the pipeline, and many other aspects of the workspace.',
						slug: '/commands/query',
						keywords: ['cli', 'commands', 'query'],
					},
				},
				'commands/run',
				'commands/setup',
				'commands/sync',
				'commands/teardown',
			],
			link: {
				type: 'generated-index',
				title: 'Commands',
				slug: '/commands',
				keywords: ['cli', 'commands'],
			},
		},
		'comparison',
		'terminology',
		'faq',
		{
			type: 'link',
			label: 'Changelog',
			href: 'https://github.com/moonrepo/moon/releases',
		},
	],
};

module.exports = sidebars;
