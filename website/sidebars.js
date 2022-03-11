/* eslint-disable sort-keys */
// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
	docs: [
		{
			type: 'category',
			label: 'Concepts',
			items: ['concepts/workspace', 'concepts/project', 'concepts/task'],
		},
		{
			type: 'category',
			label: 'Config files',
			items: ['config/workspace', 'config/global-project', 'config/project'],
		},
	],
};

// eslint-disable-next-line import/no-commonjs
module.exports = sidebars;
