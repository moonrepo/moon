import React from 'react';

export interface ImageProps {
	src: { default: string };
	width?: string;
	alt?: string;
	title?: string;
	align?: 'center' | 'left' | 'right';
	padding?: string;
}

export default function Image({
	src,
	width = '90%',
	alt = '',
	title,
	align = 'center',
	padding = '1rem',
}: ImageProps) {
	return (
		<div style={{ paddingBottom: padding, paddingTop: padding, textAlign: align }}>
			<img src={src.default} width={width} alt={alt} title={title} />
		</div>
	);
}
