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
	muted: 'text-gray-600 hover:text-gray-800 dark:hover:text-gray-400',
	primary: 'text-purple-400 hover:text-purple-300',
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
