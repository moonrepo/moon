import Label from '../ui/typography/Label';

export type StatusType = 'coming-soon' | 'experimental' | 'in-development' | 'new' | 'stable';

export interface FeatureStatusProps {
	className?: string;
	status?: StatusType;
}

export default function FeatureStatus({ className, status }: FeatureStatusProps) {
	if (!status) {
		return null;
	}

	switch (status) {
		case 'experimental':
			return <Label className={className} text="Experimental" variant="failure" />;
		case 'in-development':
			return <Label className={className} text="In development" variant="success" />;
		case 'coming-soon':
			return <Label className={className} text="Coming soon" variant="warning" />;
		case 'new':
			return <Label className={className} text="New" variant="info" />;
		// eslint-disable-next-line unicorn/no-useless-switch-case
		case 'stable':
		default:
			return null;
	}
}
