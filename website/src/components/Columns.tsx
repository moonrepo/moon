import clsx from 'clsx';

export interface ColumnsProps {
	children: React.ReactNode;
	count: 2 | 3;
}

export default function Columns({ children, count }: ColumnsProps) {
	return (
		<section
			className={clsx('sm:grid gap-3', count === 2 && 'grid-cols-2', count === 3 && 'grid-cols-3')}
		>
			{children}
		</section>
	);
}
