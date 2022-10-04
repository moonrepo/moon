import React from 'react';
import { faCode } from '@fortawesome/pro-regular-svg-icons';
import Icon from '../../ui/iconography/Icon';

export interface HeadingApiLinkProps {
	to: string;
}

export default function HeadingApiLink({ to }: HeadingApiLinkProps) {
	return (
		<a href={to} target="_blank" className="float-right inline-block" style={{ marginTop: '-3em' }}>
			<Icon icon={faCode} />
		</a>
	);
}
