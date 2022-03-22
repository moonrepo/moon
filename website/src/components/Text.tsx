import React from 'react';
import cx from 'clsx';

export type TextVariant = 'muted';

export type TextSize = 'small' | 'large';

export interface TextProps {
	as?: string;
	children: React.ReactNode;
	className?: string;
	size?: TextSize;
	variant?: TextVariant;
}

const sizes: Record<TextSize, string> = {
	small: 'text-sm',
	large: 'text-lg',
};

const variants: Record<TextVariant, string> = {
	muted: 'text-gray-600',
};

export default function Text({ as = 'p', children, className = '', size, variant }: TextProps) {
	const Tag = as as 'p';

	return (
		<Tag className={cx('text-base', size && sizes[size], variant && variants[variant], className)}>
			{children}
		</Tag>
	);
}
