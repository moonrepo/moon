import React from 'react';
import Label from '../Label';

export type StatusType = 'coming-soon' | 'experimental' | 'in-development' | 'stable';

export interface FeatureStatusProps {
	status?: StatusType;
}

export default function FeatureStatus({ status }: FeatureStatusProps) {
	switch (status) {
		case 'experimental':
			return <Label text="Experimental" variant="failure" />;
		case 'in-development':
			return <Label text="In development" variant="success" />;
		case 'coming-soon':
			return <Label text="Coming soon" variant="warning" />;
		default:
			return null;
	}
}
