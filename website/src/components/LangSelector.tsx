import React, { useCallback, useState } from 'react';

export default function LangSelector() {
	const [lang, setLang] = useState(window.localStorage?.getItem('moonrepo.language') ?? 'node');

	const handleChange = useCallback(({ target }: React.ChangeEvent<HTMLSelectElement>) => {
		const nextLang = target.value;

		setLang(nextLang);

		// Persist between sessions
		if (window.localStorage) {
			try {
				window.localStorage.setItem('moonrepo.language', nextLang);
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
			<option value="deno">Deno</option>
			<option value="node">Node</option>
		</select>
	);
}
