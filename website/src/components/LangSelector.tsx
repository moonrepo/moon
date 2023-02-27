import React, { useCallback, useEffect, useState } from 'react';
import { useLocation } from '@docusaurus/router';

const STARTED_ROUTES = [
	'install',
	'setup-workspace',
	'setup-toolchain',
	'create-project',
	'create-task',
	'run-task',
	'migrate-to-moon',
];

function hasLocalStorage() {
	return typeof window !== 'undefined' && 'localStorage' in window;
}

export function getSelectedLanguage() {
	return (hasLocalStorage() && localStorage.getItem('moonrepo.language')) || 'node';
}

export function useSelectedLanguage() {
	const [lang, setLang] = useState(getSelectedLanguage());

	useEffect(() => {
		const handler = (event: Event) => {
			setLang((event as CustomEvent<string>).detail);
		};

		window.addEventListener('onMoonrepoChangeLanguage', handler);

		return () => {
			window.removeEventListener('onMoonrepoChangeLanguage', handler);
		};
	});

	return lang;
}

export default function LangSelector() {
	const [lang, setLang] = useState(getSelectedLanguage());
	const location = useLocation();

	const handleChange = useCallback(({ target }: React.ChangeEvent<HTMLSelectElement>) => {
		const nextLang = target.value;

		setLang(nextLang);

		// Persist between sessions
		if (hasLocalStorage()) {
			try {
				localStorage.setItem('moonrepo.language', nextLang);
			} catch {
				// Ignore
			}
		}

		// Dispatch an event so markdown pages re-render
		window.dispatchEvent(
			new CustomEvent('onMoonrepoChangeLanguage', { bubbles: true, detail: nextLang }),
		);
	}, []);

	const isGettingStarted = STARTED_ROUTES.some((route) => location.pathname.endsWith(route));

	if (!isGettingStarted) {
		return null;
	}

	return (
		<select
			value={lang}
			onChange={handleChange}
			className="outline-none min-w-0 bg-white border border-solid border-gray-400 dark:border-transparent rounded-md p-0.5 text-sm text-gray-800 placeholder-gray-600 h-full font-sans"
		>
			<option value="deno">Deno</option>
			<option value="go">Go</option>
			<option value="node">Node.js</option>
			<option value="php">PHP</option>
			<option value="python">Python</option>
			<option value="ruby">Ruby</option>
			<option value="rust">Rust</option>
		</select>
	);
}
