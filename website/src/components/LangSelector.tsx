import React, { useCallback, useEffect, useState } from 'react';

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

	return (
		<select
			value={lang}
			onChange={handleChange}
			className="outline-none min-w-0 bg-white border border-solid border-gray-400 dark:border-transparent rounded-md p-0.5 text-sm text-gray-800 placeholder-gray-600 h-full font-sans"
		>
			<option value="go">Go</option>
			<option value="node">Node.js</option>
			<option value="python">Python</option>
			<option value="ruby">Ruby</option>
			<option value="rust">Rust</option>
		</select>
	);
}
