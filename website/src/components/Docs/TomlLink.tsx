import { faPuzzle } from '@fortawesome/pro-regular-svg-icons';
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
			<Label text="TOML" icon={faPuzzle} variant="info" />
		</a>
	);
}
