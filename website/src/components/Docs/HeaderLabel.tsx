import Label, { LabelProps } from '../../ui/typography/Label';

export type HeaderLabelProps = Pick<LabelProps, 'text'>;

export default function HeaderLabel({ text }: HeaderLabelProps) {
	return (
		<Label
			text={text}
			icon="material-symbols:clock-loader-40"
			variant="success"
			className="absolute right-0 top-1.5"
		/>
	);
}
