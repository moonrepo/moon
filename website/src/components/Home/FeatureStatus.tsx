import React from 'react';
import Label from '../../ui/typography/Label';

export type StatusType = 'coming-soon' | 'experimental' | 'in-development' | 'stable';

export interface FeatureStatusProps {
	className?: string;
	status?: StatusType;
}

export default function FeatureStatus({ className, status }: FeatureStatusProps) {
	switch (status) {
		case 'experimental':
			return <Label className={className} text="Experimental" variant="failure" />;
		case 'in-development':
			return <Label className={className} text="In development" variant="success" />;
		case 'coming-soon':
			return <Label className={className} text="Coming soon" variant="warning" />;
		default:
			return null;
	}
}
