import React from 'react';
import cx from 'clsx';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Icon from './Icon';

export type LabelVariant = 'failure' | 'success' | 'warning';

export interface LabelProps {
	className?: string;
	icon?: IconDefinition;
	text: string;
	variant?: LabelVariant;
}

const variants: Record<LabelVariant, string> = {
	failure: 'bg-red-100 text-red-800',
	success: 'bg-green-100 text-green-800',
	warning: 'bg-yellow-100 text-yellow-800',
};

export default function Label({ className, icon, text, variant }: LabelProps) {
	return (
		<span
			className={cx(
				'inline-flex items-center px-2 py-1 rounded text-xs font-bold uppercase',
				variant ? variants[variant] : 'bg-gray-100 text-gray-800',
				className,
			)}
		>
			{icon && <Icon icon={icon} className="mr-2" />}

			{text}
		</span>
	);
}
