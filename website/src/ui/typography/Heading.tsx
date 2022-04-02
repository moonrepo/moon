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
	1: 'text-4xl font-extrabold text-gray-900 sm:text-5xl',
	2: 'text-3xl font-extrabold text-gray-900 sm:text-4xl',
	3: 'text-2xl font-bold text-gray-900',
	4: 'text-xl font-semibold leading-8 text-gray-900',
	5: 'text-lg font-medium leading-6 text-gray-900',
	6: 'text-base font-medium text-gray-900',
};

export default function Heading({
	align = 'start',
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
				alignment[align],
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
