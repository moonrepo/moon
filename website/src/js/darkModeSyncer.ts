// Docusaurus triggers dark mode through the `data-theme="dark"` attribute
// on the `html` element, while Tailwind uses a `dark` class. This code
// listens to the `data-theme` attribute for changes, and updates the class
// name accordingly.

const html = document.documentElement;

function toggle() {
	if (html.dataset.theme === 'dark') {
		html.classList.add('dark');
	} else {
		html.classList.remove('dark');
	}
}

document.addEventListener('DOMContentLoaded', toggle);

window.addEventListener('popstate', toggle);

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
