import Label from '../../ui/typography/Label';

interface VersionLabelProps {
	header?: boolean;
	inline?: boolean;
	updated?: string;
	version: string;
}

export default function VersionLabel({ header, inline, updated, version }: VersionLabelProps) {
	return (
		<Label
			text={`v${version}`}
			variant={updated ? 'success' : 'info'}
			className={header ? 'absolute right-0 top-1.5' : inline ? 'inline-block ml-1' : 'ml-2'}
		/>
	);
}
