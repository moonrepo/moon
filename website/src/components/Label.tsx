import React from 'react';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Icon from './Icon';

export interface LabelProps {
	className?: string;
	icon?: IconDefinition;
	text: string;
	variant: 'default' | 'success' | 'failure' | 'warning';
}

const variants = {
	default: 'bg-gray-100 text-gray-800',
	failure: 'bg-red-100 text-red-800',
	success: 'bg-green-100 text-green-800',
	warning: 'bg-yellow-100 text-yellow-800',
};

export default function Label({ className = '', icon, text, variant = 'default' }: LabelProps) {
	return (
		<span
			className={`inline-flex items-center px-2 py-1 rounded text-xs font-bold uppercase ${variants[variant]} ${className}`}
		>
			{icon && (
				<span className="inline-block mr-2">
					<Icon icon={icon} />
				</span>
			)}

			{text}
		</span>
	);
}
