import Label, { LabelProps } from '../../ui/typography/Label';

export type HeaderLabelProps = Pick<LabelProps, 'text'>;

export default function HeaderLabel({ text }: HeaderLabelProps) {
	return (
		<Label text={text} icon="material-symbols:nest-clock-farsight-analog" variant="success" className="absolute right-0 top-1.5" />
	);
}
