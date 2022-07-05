import React from 'react';

export interface ImageProps {
	alt?: string;
	src: { default: string };
}

export default function Image({ alt, src }: ImageProps) {
	return (
		<div className="m-3 text-center">
			<img alt={alt} src={src.default} className="block w-auto max-w-full" />
		</div>
	);
}
