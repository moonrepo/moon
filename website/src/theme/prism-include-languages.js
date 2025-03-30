import siteConfig from '@generated/docusaurus.config';

export default function prismIncludeLanguages(PrismObject) {
	const {
		themeConfig: { prism },
	} = siteConfig;
	const { additionalLanguages } = prism;

	// Prism components work on the Prism instance on the window, while prism-
	// react-renderer uses its own Prism instance. We temporarily mount the
	// instance onto window, import components to enhance it, then remove it to
	// avoid polluting global namespace.
	// You can mutate PrismObject: registering plugins, deleting languages... As
	// long as you don't re-assign it
	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	globalThis.Prism = PrismObject;

	// eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call
	additionalLanguages.forEach((lang) => {
		// eslint-disable-next-line unicorn/prefer-module
		require(`prismjs/components/prism-${lang}`);
	});

	// We need to keep the global around so that the `twig` language works!
	// delete globalThis.Prism;
}
