import cx from 'clsx';

export interface ButtonProps {
	className?: string;
	disabled?: boolean;
	label: string;
	href?: string;
	onClick?: () => void;
	id?: string;
	size?: 'df' | 'lg';
}

export default function Button({
	className,
	disabled,
	label,
	href,
	onClick,
	id,
	size,
}: ButtonProps) {
	const isLink = Boolean(href);
	const Tag = isLink ? 'a' : 'button';

	return (
		<Tag
			className={cx(
				'border border-transparent rounded-md px-2 flex items-center justify-center text-base font-bold text-white bg-blurple-400 dark:bg-purple-600',
				disabled
					? 'opacity-60'
					: 'hover:text-white hover:bg-blurple-500 dark:hover:bg-purple-500 cursor-pointer',
				size === 'lg' ? 'py-2' : 'py-1',
				className,
			)}
			disabled={disabled}
			id={id}
			{...(isLink ? { href, target: '_blank' } : { onClick, type: 'button' })}
		>
			{label}
		</Tag>
	);
}
