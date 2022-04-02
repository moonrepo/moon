import React from 'react';
import cx from 'clsx';

export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export interface HeadingProps {
	as?: string;
	children: React.ReactNode;
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

export default function Heading({ as, children, className, level }: HeadingProps) {
	const Tag = (as ?? `h${level}`) as 'h1';

	return <Tag className={cx('m-0', levels[level], className)}>{children}</Tag>;
}
