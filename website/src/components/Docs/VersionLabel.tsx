import React from 'react';
import Label from '../../ui/typography/Label';

interface VersionLabelProps {
	header?: boolean;
	updated?: string;
	version: string;
}
export default function VersionLabel({ header, updated, version }: VersionLabelProps) {
	return (
		<Label
			text={`v${version}`}
			variant={updated ? 'success' : 'info'}
			className={header ? 'absolute right-0 top-1.5' : 'ml-2'}
		/>
	);
}
