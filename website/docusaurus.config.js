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
	title: 'moon',
	tagline: 'A build system for the JavaScript ecosystem',
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
						'moon, repo, build, system, ci, times, devx, developer, experience, tooling, tools',
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
						type: 'doc',
						docId: 'intro',
						position: 'left',
						label: 'Docs',
					},
					{
						to: '/blog',
						label: 'Blog',
						position: 'left',
					},
					{
						to: 'api',
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
					},
					{
						...social[0],
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
								label: 'Documentation',
								to: '/docs',
							},
							{
								label: 'API',
								to: '/api',
							},
							{
								label: 'Examples repository',
								href: 'https://github.com/moonrepo/examples',
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
								label: 'Configurations',
								href: 'https://github.com/moonrepo/dev',
							},
						],
					},
					{
						title: 'Support',
						items: social,
					},
				],
				copyright: `Copyright © ${new Date().getFullYear()} moon. moonrepo LLC.`,
			},
			prism: {
				theme: prismTheme,
				darkTheme: prismTheme,
			},
		}),

	plugins: [
		[
			'docusaurus-plugin-typedoc-api',
			{
				projectRoot: path.join(__dirname, '..'),
				packages: ['packages/runtime', 'packages/types'],
				minimal: true,
				readme: true,
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
