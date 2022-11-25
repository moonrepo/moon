import React from 'react';

export interface ComparisonColumnProps {
	left: React.ReactNode;
	right: React.ReactNode;
}

export default function ComparisonColumn({ left, right }: ComparisonColumnProps) {
	return (
		<section className="grid grid-cols-4 mb-4">
			<div className="col-span-2 pr-2">{left}</div>
			<div className="col-span-2 pl-2 border-0 border-l-2 border-solid border-slate-50">
				{right}
			</div>
		</section>
	);
}
