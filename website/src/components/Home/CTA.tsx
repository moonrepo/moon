import React from 'react';
import Link from '@docusaurus/Link';

export interface CTAProps {
	children: React.ReactNode;
	href: string;
}

export default function CTA({ children, href }: CTAProps) {
	return (
		<Link
			href={href}
			className="inline-flex items-center justify-center px-2 py-1 sm:px-3 sm:py-2 text-base font-bold rounded-md text-white hover:text-white bg-purple-600 hover:scale-110 md:text-lg transition-transform"
		>
			{children}
		</Link>
	);
}
