import React from 'react';
import cx from 'clsx';
import {
	TypographyAlign,
	TypographyOverflow,
	TypographyProps,
	TypographyTransform,
	TypographyVariant,
	TypographyWeight,
} from './types';

export * from './types';

export type TextElement =
	| 'abbr'
	| 'b'
	| 'bdo'
	| 'cite'
	| 'code'
	| 'data'
	| 'dd'
	| 'dfn'
	| 'div'
	| 'em'
	| 'i'
	| 'kbd'
	| 'p'
	| 'q'
	| 'samp'
	| 'small'
	| 'span'
	| 'strong'
	| 'sub'
	| 'sup'
	| 'time'
	| 'var'
	| 'wbr';

export type TextSize = 'df' | 'lg' | 'sm';

export interface TextProps extends TypographyProps {
	as?: TextElement;
	className?: string;
	size?: TextSize;
}

export const alignment: Record<TypographyAlign, string> = {
	center: 'text-center',
	end: 'text-right',
	justify: 'text-justify',
	start: 'text-left',
};

export const overflows: Record<TypographyOverflow, string> = {
	clip: 'text-clip',
	ellipsis: 'text-ellipsis',
	truncate: 'truncate',
	wrap: '',
};

export const sizes: Record<TextSize, string> = {
	df: 'text-base',
	lg: 'text-lg',
	sm: 'text-sm',
};

export const transforms: Record<TypographyTransform, string> = {
	capitalize: 'capitalize',
	lowercase: 'lowercase',
	uppercase: 'uppercase',
};

export const variants: Record<TypographyVariant, string> = {
	muted: 'text-gray-500',
	neutral: '',
};

export const weights: Record<TypographyWeight, string> = {
	black: 'font-black',
	bold: 'font-bold',
	light: 'font-light',
	medium: 'font-medium',
	normal: 'font-normal',
	thin: 'font-thin',
};

export default function Text<T extends TextElement>({
	align,
	as: Tag = 'p',
	children,
	className = '',
	overflow = 'wrap',
	size = 'df',
	transform,
	variant = 'neutral',
	weight = 'normal',
}: React.ComponentProps<T> & TextProps) {
	return (
		<Tag
			className={cx(
				'm-0',
				align && alignment[align],
				overflows[overflow],
				sizes[size],
				transform && transforms[transform],
				variants[variant],
				weights[weight],
				className,
			)}
		>
			{children}
		</Tag>
	);
}
