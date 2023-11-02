/* eslint-disable sort-keys */

import type { PrismTheme } from 'prism-react-renderer';
import tailwind from './tailwind.config';

const colors = tailwind.theme!.colors as Record<string, string>;

const theme: PrismTheme = {
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

export default theme;
