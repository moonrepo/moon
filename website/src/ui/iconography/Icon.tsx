import React, { CSSProperties } from 'react';
import cx from 'clsx';
import { Icon as BaseIcon, IconProps as BaseIconProps } from '@iconify/react';

export interface IconProps extends BaseIconProps {
	className?: string;
	style?: CSSProperties;
}

export default function Icon({ className, style, ...props }: IconProps) {
	return (
		<span className={cx('inline-block', className)} aria-hidden="true" style={style}>
			<BaseIcon {...props} />
		</span>
	);
}
