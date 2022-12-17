import React, { Children } from 'react';
import Admonition from '@theme/Admonition';
import { useSelectedLanguage } from './LangSelector';

const LABELS: Record<string, string> = {
	deno: 'Deno',
	node: 'Node.js',
};

export interface LangPartialsProps {
	children: React.ReactNode;
}

export default function LangPartials({ children }: LangPartialsProps) {
	const lang = useSelectedLanguage();

	const selected = Children.toArray(children).find((child) => {
		if (React.isValidElement(child)) {
			return typeof child.key === 'string' && child.key.endsWith(lang);
		}

		return false;
	});

	if (!selected) {
		return (
			<Admonition type="danger">
				Sorry, there's no example for {LABELS[lang] || lang}. Try switching to another language for
				the time being!
			</Admonition>
		);
	}

	return <>{selected}</>;
}
