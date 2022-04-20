import React from 'react';
import cx from 'clsx';
import { alignment, overflows, transforms, variants } from './Text';
import { TypographyProps } from './types';

export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export type HeadingElement = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' | 'p';

export interface HeadingProps extends Omit<TypographyProps, 'weight'> {
	as?: HeadingElement;
	className?: string;
	level: HeadingLevel;
}

const levels: Record<HeadingLevel, string> = {
	1: 'text-4xl font-extrabold sm:text-5xl',
	2: 'text-3xl font-extrabold sm:text-4xl',
	3: 'text-2xl font-bold',
	4: 'text-xl font-bold',
	5: 'text-lg font-semibold',
	6: 'text-base font-semibold',
};

export default function Heading({
	align,
	as,
	children,
	className = '',
	level,
	overflow = 'wrap',
	transform,
	variant = 'neutral',
}: HeadingProps) {
	const Tag = (as ?? `h${level}`) as 'h1';

	return (
		<Tag
			className={cx(
				'm-0',
				align && alignment[align],
				levels[level],
				overflows[overflow],
				transform && transforms[transform],
				variants[variant],
				className,
			)}
		>
			{children}
		</Tag>
	);
}
