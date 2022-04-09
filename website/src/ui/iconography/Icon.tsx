import React from 'react';
import cx from 'clsx';
import { FontAwesomeIcon, FontAwesomeIconProps } from '@fortawesome/react-fontawesome';

export interface IconProps extends FontAwesomeIconProps {
	className?: string;
}

export default function Icon({ className, ...props }: IconProps) {
	return (
		<span className={cx('inline-block', className)} aria-hidden="true">
			<FontAwesomeIcon {...props} />
		</span>
	);
}
