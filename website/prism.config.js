const tailwind = require('./tailwind.config');

const { colors } = tailwind.theme;

module.exports = {
	plain: {
		backgroundColor: colors.slate['900'],
		color: colors.gray['100'],
	},
	styles: [
		{
			types: ['changed'],
			style: {
				color: colors.yellow['100'],
			},
		},
		{
			types: ['deleted'],
			style: {
				color: colors.red['300'],
			},
		},
		{
			types: ['inserted'],
			style: {
				color: colors.green['300'],
			},
		},
		{
			types: ['comment'],
			style: {
				color: colors.gray['600'],
				fontStyle: 'italic',
			},
		},
		{
			types: ['punctuation'],
			style: {
				color: colors.gray['300'],
			},
		},
		{
			types: ['constant'],
			style: {
				color: colors.red['200'],
			},
		},
		{
			types: ['string', 'url'],
			style: {
				color: colors.green['200'],
			},
		},
		{
			types: ['variable'],
			style: {
				color: colors.yellow['100'],
			},
		},
		{
			types: ['number', 'boolean'],
			style: {
				color: colors.teal['300'],
			},
		},
		{
			types: ['attr-name'],
			style: {
				color: colors.yellow['300'],
			},
		},
		{
			types: ['keyword', 'operator', 'property', 'namespace', 'tag', 'selector', 'doctype'],
			style: {
				color: colors.purple['300'],
			},
		},
		{
			types: ['builtin', 'char', 'constant', 'function', 'class-name'],
			style: {
				color: colors.pink['300'],
				fontWeight: 'bold',
			},
		},
	],
};
