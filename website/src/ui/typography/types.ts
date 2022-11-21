export type TypographyAlign = 'center' | 'end' | 'justify' | 'start';

export type TypographyOverflow = 'clip' | 'ellipsis' | 'truncate' | 'wrap';

export type TypographyTransform = 'capitalize' | 'lowercase' | 'uppercase';

export type TypographyWeight = 'black' | 'bold' | 'light' | 'medium' | 'normal' | 'thin';

export type TypographyVariant = 'muted' | 'neutral';

export interface TypographyProps {
	/**
	 * Align the text on the horizontal axis.
	 * @default start
	 */
	align?: TypographyAlign;
	/**
	 * String of text to display.
	 */
	children: React.ReactNode;
	/**
	 * Customize how the text will overflow its current container.
	 * @default wrap
	 */
	overflow?: TypographyOverflow;
	/**
	 * Apply a transformation to the entire string of text.
	 */
	transform?: TypographyTransform;
	/**
	 * Customize the text color based on the current design system theme.
	 * @default neutral
	 */
	variant?: TypographyVariant;
	/**
	 * Apply a light or bold weight to the entire string of text.
	 * @default normal
	 */
	weight?: TypographyWeight;
}
