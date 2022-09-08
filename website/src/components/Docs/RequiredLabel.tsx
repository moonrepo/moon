import React from 'react';
import Label from '../../ui/typography/Label';

export interface RequiredLabelProps {
	text?: string;
}

export default function RequiredLabel({ text = 'Required' }: RequiredLabelProps) {
	return <Label text={text} variant="failure" className="ml-2" />;
}
