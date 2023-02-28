import React from 'react';

export interface HeroIconProps {
	icon: React.ReactNode;
	text: React.ReactNode;
}

export default function HeroIcon({ icon, text }: HeroIconProps) {
	return (
		<h1
			className="text-white flex justify-center md:justify-start items-stretch gap-3"
			style={{ height: 85 }}
		>
			<div className="relative">{icon}</div>
			<div className="relative">{text}</div>
		</h1>
	);
}
