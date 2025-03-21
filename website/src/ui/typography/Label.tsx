import cx from 'clsx';
import type { IconifyIcon } from '@iconify/react';
import Icon from '../iconography/Icon';

export type LabelVariant = 'failure' | 'info' | 'success' | 'warning';

export interface LabelProps {
	className?: string;
	icon?: IconifyIcon | string;
	text: string;
	variant?: LabelVariant;
}

const variants: Record<LabelVariant, string> = {
	failure: 'bg-red-100 text-red-900',
	info: 'bg-pink-100 text-pink-900',
	success: 'bg-green-100 text-green-900',
	warning: 'bg-orange-100 text-orange-900',
};

export default function Label({ className, icon, text, variant }: LabelProps) {
	return (
		<span
			className={cx(
				'inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase',
				variant ? variants[variant] : 'bg-gray-100 text-gray-800',
				className,
			)}
		>
			{icon && <Icon icon={icon} className="mr-1" />}

			{text}
		</span>
	);
}
