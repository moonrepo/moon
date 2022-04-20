import React from 'react';
import cx from 'clsx';
import BaseLink, { Props as BaseLinkProps } from '@docusaurus/Link';
import { sizes, TextSize, transforms, weights } from './Text';
import { TypographyProps } from './types';

export type LinkVariant = 'muted' | 'primary';

export interface LinkProps extends BaseLinkProps, Pick<TypographyProps, 'transform' | 'weight'> {
	size?: TextSize;
	variant?: LinkVariant;
}

const variants: Record<LinkVariant, string> = {
	muted: 'text-gray-700 hover:text-gray-800 dark:text-gray-600 dark:hover:text-gray-500',
	primary:
		'text-blurple-400 hover:text-blurple-600 dark:text-purple-400 dark:hover:text-purple-200',
};

export default function Link({
	className,
	transform,
	size = 'df',
	weight = 'normal',
	variant = 'primary',
	...props
}: LinkProps) {
	return (
		<BaseLink
			className={cx(
				sizes[size],
				transform && transforms[transform],
				variants[variant],
				weights[weight],
				className,
			)}
			{...props}
		/>
	);
}
