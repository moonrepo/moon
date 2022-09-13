import React from 'react';

export interface ImageProps {
	src: { default: string };
	width?: string;
	alt?: string;
	title?: string;
}

export default function Image({ src, width = '90%', alt = '', title }: ImageProps) {
	return (
		<div style={{ paddingBottom: '1rem', paddingTop: '1rem', textAlign: 'center' }}>
			<img src={src.default} width={width} alt={alt} title={title} />
		</div>
	);
}
