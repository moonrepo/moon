import React from 'react';
import clsx from 'clsx';

export interface TwoColumnProps {
	aside: React.ReactNode;
	children: React.ReactNode;
	reversed?: boolean;
}

export default function TwoColumn({ children, aside, reversed }: TwoColumnProps) {
	return (
		<section className="sm:grid gap-2 grid-cols-5 mb-4">
			<div className={clsx('col-span-3', reversed && 'order-2')}>{children}</div>
			<aside className={clsx('col-span-2', reversed && 'order-1')}>{aside}</aside>
		</section>
	);
}
