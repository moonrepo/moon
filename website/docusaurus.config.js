/* eslint-disable sort-keys */
// @ts-check

const path = require('path');
const prismTheme = require('./prism.config');

const social = [
	{
		label: 'GitHub',
		to: 'https://github.com/moonrepo/moon',
	},
	{
		label: 'Discord',
		to: 'https://discord.gg/qCh9MEynv2',
	},
	{
		label: 'Twitter',
		to: 'https://twitter.com/tothemoonrepo',
	},
];

/** @type {import('@docusaurus/types').Config} */
const config = {
	title: 'moonrepo',
	tagline: 'A developer productivity platform for scaling codebases.',
	url: 'https://moonrepo.dev',
	baseUrl: '/',
	onBrokenLinks: 'throw',
	onBrokenMarkdownLinks: 'warn',
	favicon: 'img/favicon.svg',
	organizationName: 'moonrepo',
	projectName: 'moon',
	deploymentBranch: 'gh-pages',
	trailingSlash: false,

	presets: [
		[
			'classic',
			/** @type {import('@docusaurus/preset-classic').Options} */
			({
				docs: {
					sidebarPath: require.resolve('./sidebars.js'),
					editUrl: 'https://github.com/moonrepo/moon/tree/master/website',
				},
				blog: {
					showReadingTime: true,
					editUrl: 'https://github.com/moonrepo/moon/tree/master/website',
				},
				theme: {
					customCss: [
						require.resolve('./src/css/theme.css'),
						require.resolve('./src/css/custom.css'),
					],
				},
				gtag: {
					trackingID: 'G-LB233GTZD3',
				},
			}),
		],
	],

	themeConfig:
		/** @type {import('@docusaurus/preset-classic').ThemeConfig} */
		({
			algolia: {
				apiKey: 'dfe3e44100d7dfc6d7d3b644e8b09581',
				appId: '400S075OEM',
				indexName: 'moonrepo',
			},
			metadata: [
				{
					name: 'keywords',
					content:
						'moon, repo, build, system, ci, times, devx, developer, experience, tooling, tools, monorepo, polyrepo, productivity, platform',
				},
				{
					name: 'og:image',
					content: 'https://moonrepo.dev/img/hero/slate-bg.jpg',
				},
			],
			navbar: {
				// title: 'moon',
				logo: {
					alt: 'moon',
					src: 'img/logo.svg',
				},
				items: [
					{
						type: 'dropdown',
						position: 'left',
						label: 'Products',
						items: [
							{
								to: '/moon',
								html: '<strong class="block mr-2">moon</strong><div class="opacity-60">Build system for managing codebases</div>',
							},
							{
								to: '/moonbase',
								html: '<strong class="block mr-2">moonbase</strong><div class="opacity-60">Service for scaling CI pipelines</div>',
							},
							// {
							// 	to: '/proto',
							// 	html: '<strong>proto</strong><div class="opacity-60">Language agnostic version manager</div>',
							// },
						],
					},
					{
						type: 'doc',
						docId: 'intro',
						position: 'left',
						label: 'Docs',
					},
					{
						type: 'doc',
						docId: 'guides/ci',
						position: 'left',
						label: 'Guides',
					},
					{
						to: '/blog',
						label: 'Blog',
						position: 'left',
					},
					{
						...social[0],
						position: 'left',
					},
					/* {
						to: '/api',
						label: 'API',
						position: 'left',
					},
					{
						type: 'dropdown',
						label: 'Packages',
						items: [
							{
								label: '@moonrepo/cli',
								href: 'https://www.npmjs.com/package/@moonrepo/cli',
							},
							{
								label: '@moonrepo/dev',
								href: 'https://www.npmjs.com/package/@moonrepo/dev',
							},
							{
								label: '@moonrepo/report',
								href: 'https://www.npmjs.com/package/@moonrepo/report',
							},
							{
								label: '@moonrepo/types',
								href: 'https://www.npmjs.com/package/@moonrepo/types',
							},
							{
								label: 'babel-preset-moon',
								href: 'https://www.npmjs.com/package/babel-preset-moon',
							},
							{
								label: 'eslint-config-moon',
								href: 'https://www.npmjs.com/package/eslint-config-moon',
							},
							{
								label: 'jest-preset-moon',
								href: 'https://www.npmjs.com/package/jest-preset-moon',
							},
							{
								label: 'prettier-config-moon',
								href: 'https://www.npmjs.com/package/prettier-config-moon',
							},
							{
								label: 'tsconfig-moon',
								href: 'https://www.npmjs.com/package/tsconfig-moon',
							},
						],
					}, */
					{
						to: 'https://moonrepo.app',
						label: 'Sign in',
						position: 'right',
					},
				],
			},
			footer: {
				style: 'dark',
				links: [
					{
						title: 'Learn',
						items: [
							{
								label: 'Docs',
								to: '/docs',
							},
							{
								label: 'Guides',
								to: '/docs/guides/ci',
							},
							{
								label: 'Blog',
								to: '/blog',
							},
							{
								label: 'API',
								to: '/api',
							},
						],
					},
					{
						title: 'Ecosystem',
						items: [
							{
								label: 'Releases',
								to: 'https://github.com/moonrepo/moon/releases',
							},
							{
								label: 'Discussions',
								to: 'https://github.com/moonrepo/moon/discussions',
							},
							{
								label: 'Developer tools',
								href: 'https://github.com/moonrepo/dev',
							},
							{
								label: 'Examples repository',
								href: 'https://github.com/moonrepo/examples',
							},
						],
					},
					{
						title: 'Support',
						items: social,
					},
				],
				copyright: `Copyright © ${new Date().getFullYear()}, moonrepo LLC`,
			},
			prism: {
				theme: prismTheme,
				darkTheme: prismTheme,
				additionalLanguages: ['docker', 'twig'],
			},
		}),

	plugins: [
		[
			'@docusaurus/plugin-client-redirects',
			{
				redirects: [
					{
						from: '/docs/config/global-project',
						to: '/docs/config/tasks',
					},
					{
						from: '/docs/config/inherited-tasks',
						to: '/docs/config/tasks',
					},
				],
			},
		],
		[
			'docusaurus-plugin-typedoc-api',
			{
				projectRoot: path.join(__dirname, '..'),
				packages: ['packages/report', 'packages/runtime', 'packages/types'],
				minimal: true,
				readmes: true,
			},
		],
		function tailwind() {
			return {
				name: 'docusaurus-tailwindcss',
				configurePostCss(postcssOptions) {
					// eslint-disable-next-line import/no-extraneous-dependencies
					postcssOptions.plugins.push(require('tailwindcss'));

					return postcssOptions;
				},
			};
		},
	],

	clientModules: [require.resolve('./src/js/darkModeSyncer.ts')],
};

module.exports = config;
