/* eslint-disable sort-keys */

import React from 'react';
import { faLeaf, faScrewdriverWrench, faSolarSystem } from '@fortawesome/pro-duotone-svg-icons';
import { faBolt } from '@fortawesome/pro-regular-svg-icons';
import Features, { Feature } from '@site/src/components/Products/Features';
import Hero from '@site/src/components/Products/Proto/Hero';
import Layout from '@theme/Layout';

const toolchainFeatures: Feature[] = [
	{
		title: 'Lightspeed commands',
		icon: faBolt,
		description:
			'Download, install, and run tools with lightspeed, thanks to our Rust based foundation.',
	},
	{
		title: 'Universal toolchain',
		icon: faSolarSystem,
		description: 'Manage multiple languages and dependency managers through a single command.',
	},
	{
		title: 'Ecosystem aware',
		icon: faLeaf,
		description: "Detects and infers from a language's ecosystem for maximum compatibility.",
	},
	{
		title: 'Granular configuration',
		icon: faScrewdriverWrench,
		description: 'Configure tools and their versions per directory, or per project.',
	},
];

export default function ProductProto() {
	return (
		<Layout
			title="proto - A language agnostic toolchain manager"
			description="Lightspeed toolchain manager for programming languages and their dependency managers."
		>
			<Hero />

			<main>
				<div className="bg-gradient-to-b from-slate-600 via-pink-900 to-white">
					<Features
						header="Toolchain"
						description="All in one"
						features={toolchainFeatures}
						tier={2}
					/>

					<div className="relative py-4 sm:py-5 lg:py-6">
						<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
							<div className="bg-white rounded-lg p-6 drop-shadow">sdsds</div>
						</div>
					</div>
				</div>
			</main>
		</Layout>
	);
}
