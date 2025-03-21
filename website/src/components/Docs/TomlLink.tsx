import Label from '../../ui/typography/Label';

export interface TomlLinkProps {
	to: string;
	noMargin?: boolean;
}

export default function TomlLink({ to, noMargin }: TomlLinkProps) {
	return (
		<a
			href={to}
			target="_blank"
			className="float-right block"
			style={{ marginTop: noMargin ? 0 : '-3.75em' }}
		>
			<Label text="TOML" icon="material-symbols:extension" variant="info" />
		</a>
	);
}
