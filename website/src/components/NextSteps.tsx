import React from 'react';
import Link from '@docusaurus/Link';
import ProductIcon, { ProductIconName } from '../ui/iconography/ProductIcon';

export interface NextStepsProps {
	links: { icon: ProductIconName; label: React.ReactNode; url: string }[];
}

export default function NextSteps({ links }: NextStepsProps) {
	return (
		<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
			{links.map((link) => (
				<Link key={link.url} href={link.url} className="focus:outline-none">
					<div className="relative rounded-lg px-3 py-3 flex items-center space-x-2 border-solid border border-t-0 border-b-2 bg-gray-50 hover:bg-gray-100/50 border-gray-200/75 dark:bg-slate-700 dark:hover:bg-slate-600 dark:border-slate-900/75">
						<div className="flex-shrink-0">
							<ProductIcon size="lg" name={link.icon} />
						</div>

						<div className="flex-1 min-w-0 text-gray-900 dark:text-gray-100">{link.label}</div>
					</div>
				</Link>
			))}
		</div>
	);
}
