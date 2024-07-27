import clsx from 'clsx';

export interface HeroIconProps {
	icon: React.ReactNode;
	text: React.ReactNode;
	textClass?: string;
}

export default function HeroIcon({ icon, text, textClass }: HeroIconProps) {
	return (
		<h1
			className="text-white flex justify-center md:justify-start items-stretch gap-3"
			style={{ height: 85 }}
		>
			<div className="relative">{icon}</div>
			<div className={clsx('relative', textClass)}>{text}</div>
		</h1>
	);
}
