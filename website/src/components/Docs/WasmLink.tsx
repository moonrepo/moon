import React from 'react';
import { faPuzzle } from '@fortawesome/pro-regular-svg-icons';
import Label from '../../ui/typography/Label';

export interface WasmLinkProps {
	to: string;
}

export default function WasmLink({ to }: WasmLinkProps) {
	return (
		<a href={to} target="_blank" className="float-right block" style={{ marginTop: '-3.75em' }}>
			<Label text="WASM plugin" icon={faPuzzle} variant="success" />
		</a>
	);
}
