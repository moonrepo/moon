import React from 'react';
import cx from 'clsx';

export interface ButtonProps {
	disabled?: boolean;
	label: string;
	href?: string;
	onClick?: () => void;
	id?: string;
}

export default function Button({ disabled, label, href, onClick, id }: ButtonProps) {
	const isLink = !!href;
	const Tag = isLink ? 'a' : 'button';

	return (
		<Tag
			className={cx(
				'w-1/4 border border-transparent rounded-md px-2 py-1 flex items-center justify-center text-base font-bold text-white bg-blurple-400 dark:bg-purple-600',
				disabled
					? 'opacity-60'
					: 'hover:text-white hover:bg-blurple-500 dark:hover:bg-purple-500 cursor-pointer',
			)}
			disabled={disabled}
			id={id}
			{...(isLink ? { href, target: '_blank' } : { onClick, type: 'button' })}
		>
			{label}
		</Tag>
	);
}
