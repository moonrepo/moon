import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';

export interface CTAProps {
	children: React.ReactNode;
	href: string;
	color?: string;
}

export default function CTA({ children, href, color }: CTAProps) {
	return (
		<Link
			href={href}
			className={clsx(
				'inline-flex items-center justify-center px-2 py-1 sm:px-3 sm:py-2 text-base font-bold rounded-md text-white hover:text-white hover:scale-105 md:text-lg transition-transform',
				color ?? 'bg-purple-600',
			)}
		>
			{children}
		</Link>
	);
}
