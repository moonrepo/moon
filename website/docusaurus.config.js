/* eslint-disable sort-keys */
// @ts-check

const path = require('path');
const prismTheme = require('./prism.config');

/** @type {import('@docusaurus/types').Config} */
const config = {
	title: 'Moon',
	tagline: 'Build system for JavaScript based repos.',
	url: 'https://moonrepo.dev',
	baseUrl: '/',
	onBrokenLinks: 'throw',
	onBrokenMarkdownLinks: 'warn',
	favicon: 'img/favicon.ico',
	organizationName: 'milesj',
	projectName: 'moon',

	presets: [
		[
			'classic',
			/** @type {import('@docusaurus/preset-classic').Options} */
			({
				docs: {
					sidebarPath: require.resolve('./sidebars.js'),
					editUrl: 'https://github.com/milesj/moon/tree/master/website',
				},
				// blog: {
				// 	showReadingTime: true,
				// 	// Please change this to your repo.
				// 	editUrl:
				// 		'https://github.com/milesj/moon/tree/master/website',
				// },
				theme: {
					customCss: [
						require.resolve('./src/css/icons/fontawesome.css'),
						require.resolve('./src/css/icons/solid.css'),
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
			navbar: {
				title: 'Moon',
				logo: {
					alt: 'Moon',
					src: 'img/logo.svg',
				},
				items: [
					{
						type: 'doc',
						docId: 'intro',
						position: 'left',
						label: 'Docs',
					},
					// {
					// 	to: '/blog',
					// 	label: 'Blog',
					// 	position: 'left',
					// },
					{
						to: 'api',
						label: 'Packages',
						position: 'left',
					},
					{
						href: 'https://github.com/milesj/moon',
						label: 'GitHub',
						position: 'right',
					},
				],
			},
			footer: {
				style: 'dark',
				links: [],
				copyright: `Copyright © ${new Date().getFullYear()} Moon. Built with Docusaurus.`,
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
				packages: ['packages/runtime'],
				minimal: true,
				readme: true,
			},
		],
		function tailwind() {
			return {
				name: 'docusaurus-tailwindcss',
				configurePostCss(postcssOptions) {
					// eslint-disable-next-line import/no-extraneous-dependencies, node/no-unpublished-require
					postcssOptions.plugins.push(require('tailwindcss'));

					return postcssOptions;
				},
			};
		},
	],

	clientModules: [require.resolve('./src/js/darkModeSyncer.ts')],
};

module.exports = config;
