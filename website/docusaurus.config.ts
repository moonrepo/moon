/* eslint-disable sort-keys */

// import path from 'node:path';
import type * as Preset from '@docusaurus/preset-classic';
import type { Config } from '@docusaurus/types';
import prismTheme from './prism.config';

const social = [
	{
		label: 'GitHub',
		to: 'https://github.com/moonrepo',
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

const config: Config = {
	title: 'moonrepo',
	tagline: 'A developer productivity tooling platform.',
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
			{
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
			} satisfies Preset.Options,
		],
	],

	themeConfig: {
		algolia: {
			apiKey: 'dfe3e44100d7dfc6d7d3b644e8b09581',
			appId: '400S075OEM',
			indexName: 'moonrepo',
		},
		metadata: [
			{
				name: 'keywords',
				content:
					'moon, repo, moonrepo, task, runner, build, system, ci, times, devx, developer, experience, tooling, tools, monorepo, polyrepo, productivity, platform, proto, toolchain',
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
						// {
						// 	to: '/moonbase',
						// 	html: '<strong class="block mr-2">moonbase</strong><div class="opacity-60">Service for scaling CI pipelines</div>',
						// },
						{
							to: '/proto',
							html: '<strong class="block mr-2">proto</strong><div class="opacity-60">Multi-language version manager</div>',
						},
						// {
						// 	to: 'https://espresso.build',
						// 	html: '<strong class="block mr-2">espresso</strong><div class="opacity-60">Next-gen JavaScript package system</div>',
						// },
					],
				},
				{
					type: 'dropdown',
					position: 'left',
					label: 'Docs',
					items: [
						{
							type: 'doc',
							docId: 'intro',
							html: '<strong>moon</strong>',
						},
						{
							type: 'doc',
							docId: 'proto/index',
							html: '<strong>proto</strong>',
						},
					],
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
				// {
				// 	to: 'https://moonrepo.app',
				// 	label: 'Sign in',
				// 	position: 'right',
				// },
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
						// {
						// 	label: 'API',
						// 	to: '/api',
						// },
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
							label: 'Shared configs',
							to: 'https://github.com/moonrepo/moon-configs',
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
			copyright: `Copyright © ${new Date().getFullYear()}, moonrepo, Inc.`,
		},
		prism: {
			theme: prismTheme,
			darkTheme: prismTheme,
			additionalLanguages: [
				'bash',
				'diff',
				'docker',
				'json',
				'markup-templating',
				'rust',
				'toml',
				'twig',
				'typescript',
			],
		},
	} satisfies Preset.ThemeConfig,

	plugins: [
		[
			'@docusaurus/plugin-client-redirects',
			{
				redirects: [
					{
						from: '/docs/how-it-works/dep-graph',
						to: '/docs/how-it-works/action-graph',
					},
					{
						from: '/docs/commands/dep-graph',
						to: '/docs/commands/action-graph',
					},
					{
						from: '/docs/config/global-project',
						to: '/docs/config/tasks',
					},
					{
						from: '/docs/config/inherited-tasks',
						to: '/docs/config/tasks',
					},
					{
						from: '/docs/guides/git-hooks',
						to: '/docs/guides/vcs-hooks',
					},
					{
						from: '/docs/proto/toml-plugin',
						to: '/docs/proto/non-wasm-plugin',
					},
					{
						from: '/docs/proto/version-spec',
						to: '/docs/proto/tool-spec',
					},
				],
			},
		],
		// [
		// 	'docusaurus-plugin-typedoc-api',
		// 	{
		// 		projectRoot: path.join(__dirname, '..'),
		// 		packages: ['packages/report', 'packages/runtime', 'packages/types'],
		// 		minimal: true,
		// 		readmes: true,
		// 	},
		// ],
		function tailwind() {
			return {
				name: 'docusaurus-tailwindcss',
				configurePostCss(postcssOptions) {
					postcssOptions.plugins.push(require('tailwindcss'));

					return postcssOptions;
				},
			};
		},
	],

	clientModules: [require.resolve('./src/js/darkModeSyncer.ts')],
};

export default config;
