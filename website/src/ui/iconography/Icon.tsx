import React from 'react';
import cx from 'clsx';
import { FontAwesomeIcon, FontAwesomeIconProps } from '@fortawesome/react-fontawesome';

export interface IconProps extends FontAwesomeIconProps {
	className?: string;
	style?: unknown;
}

export default function Icon({ className, style, ...props }: IconProps) {
	return (
		<span className={cx('inline-block', className)} aria-hidden="true" style={style}>
			<FontAwesomeIcon {...props} />
		</span>
	);
}
