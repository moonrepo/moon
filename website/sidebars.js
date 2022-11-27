/* eslint-disable sort-keys */
// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
	docs: [
		'intro',
		{
			type: 'category',
			label: 'Getting started',
			collapsed: false,
			collapsible: true,
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
			items: [
				'config/workspace',
				'config/toolchain',
				'config/global-project',
				'config/project',
				'config/template',
			],
			link: {
				type: 'generated-index',
				title: 'Config files',
				slug: '/config',
				keywords: ['config'],
			},
		},
		{
			type: 'category',
			label: 'Editors',
			items: ['editors/vscode'],
			link: {
				type: 'generated-index',
				title: 'Editors',
				slug: '/editors',
				keywords: ['editors', 'vscode'],
			},
		},
		{
			type: 'category',
			label: 'Commands',
			items: [
				'commands/overview',
				'commands/bin',
				'commands/ci',
				'commands/check',
				'commands/clean',
				'commands/dep-graph',
				{
					type: 'category',
					label: 'docker',
					items: ['commands/docker/prune', 'commands/docker/scaffold'],
					link: {
						type: 'generated-index',
						title: 'docker',
						description: 'Operations for integrating with Docker and Dockerfiles.',
						slug: '/commands/docker',
						keywords: ['cli', 'commands', 'docker'],
					},
				},
				'commands/generate',
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

	guides: [
		'guides/ci',
		'guides/codegen',
		'guides/docker',
		'guides/git-hooks',
		'guides/open-source',
		'guides/profile',
		'guides/remote-cache',
		'guides/root-project',
		'guides/sharing-config',
		'guides/webhooks',
		{
			type: 'html',
			value: '<hr />',
			defaultStyle: true,
		},
		{
			type: 'category',
			label: 'JavaScript',
			collapsed: false,
			items: [
				'guides/javascript/typescript-project-refs',
				{
					type: 'category',
					label: 'Examples',
					collapsed: true,
					collapsible: true,
					items: [
						'guides/examples/astro',
						'guides/examples/eslint',
						'guides/examples/jest',
						'guides/examples/next',
						'guides/examples/nuxt',
						'guides/examples/packemon',
						'guides/examples/prettier',
						'guides/examples/react',
						'guides/examples/remix',
						'guides/examples/solid',
						'guides/examples/typescript',
						'guides/examples/vite',
						'guides/examples/vue',
					],
					link: {
						type: 'generated-index',
						title: 'Node.js examples',
						slug: '/guides/node/examples',
						keywords: ['node', 'javascript', 'typescript', 'guides', 'examples', 'tools'],
					},
				},
			],
		},
	],
};

// eslint-disable-next-line import/no-commonjs
module.exports = sidebars;
