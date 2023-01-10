import React, { Children } from 'react';
import Admonition from '@theme/Admonition';
import { useSelectedLanguage } from './LangSelector';

const LABELS: Record<string, string> = {
	bun: 'Buno',
	deno: 'Deno',
	go: 'Go',
	node: 'Node.js',
	php: 'PHP',
	python: 'Python',
	ruby: 'Ruby',
	rust: 'Rust',
};

export interface LangPartialsProps {
	children: React.ReactNode;
	noError?: boolean;
}

export default function LangPartials({ children, noError }: LangPartialsProps) {
	const lang = useSelectedLanguage();

	const selected = Children.toArray(children).find((child) => {
		if (React.isValidElement(child)) {
			return typeof child.key === 'string' && child.key.endsWith(lang);
		}

		return false;
	});

	if (!selected) {
		if (noError) {
			return null;
		}

		return (
			<Admonition type="danger">
				Sorry, there's no example for {LABELS[lang] || lang}. Try switching to another language for
				the time being!
			</Admonition>
		);
	}

	return <>{selected}</>;
}
