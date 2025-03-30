// Docusaurus triggers dark mode through the `data-theme="dark"` attribute
// on the `html` element, while Tailwind uses a `dark` class. This code
// listens to the `data-theme` attribute for changes, and updates the class
// name accordingly.

// This is necessary since it gets executed on the server???
if (typeof document !== 'undefined') {
	const html = document.documentElement;
	const app = document.querySelector('#__docusaurus')!;

	// We can't set the class on `html` or `body` as Docusaurus rewrites the classes
	function toggle() {
		if (html.dataset.theme === 'dark') {
			app.classList.add('dark');
		} else if (html.dataset.theme === 'light') {
			app.classList.remove('dark');
		}
	}

	document.addEventListener('DOMContentLoaded', toggle);

	window.history.pushState = new Proxy(window.history.pushState, {
		apply: (target, thisArg, argArray) => {
			toggle();
			// @ts-expect-error Ignore arg types
			target.apply(thisArg, argArray);
		},
	});

	window.history.replaceState = new Proxy(window.history.replaceState, {
		apply: (target, thisArg, argArray) => {
			toggle();
			// @ts-expect-error Ignore arg types
			target.apply(thisArg, argArray);
		},
	});

	const observer = new MutationObserver((mutations) => {
		for (const mutation of mutations) {
			if (mutation.type === 'attributes') {
				toggle();
			}
		}
	});

	observer.observe(html, {
		attributeFilter: ['data-theme'],
		attributes: true,
	});
}

export {};
