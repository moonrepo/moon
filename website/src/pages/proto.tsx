/* eslint-disable sort-keys */

import React from 'react';
import { faCloudArrowUp, faDiagramSankey, faTimeline } from '@fortawesome/pro-duotone-svg-icons';
import Features, { Feature } from '@site/src/components/Products/Features';
import Hero from '@site/src/components/Products/Moonbase/Hero';
import Layout from '@theme/Layout';

const ciFeatures: Feature[] = [
	{
		title: 'Artifact caching',
		icon: faCloudArrowUp,
		description: 'Cache build artifacts between CI runs to reduce job times and overall costs.',
	},
	{
		title: 'Run history',
		icon: faTimeline,
		description:
			'Track CI runs to detect flakiness, regressions, and time spent on task execution.',
		status: 'in-development',
	},
	{
		title: 'Remote distribution',
		icon: faDiagramSankey,
		description: 'Distribute task execution across multiple remote agents to increase throughput.',
		status: 'coming-soon',
	},
];

export default function ProductProto() {
	return (
		<Layout
			title="proto - A language agnostic toolchain manager"
			description="Lightning fast toolchain manager for programming languages and their dependency managers."
		>
			<Hero />

			<main>
				<div className="bg-gradient-to-b from-slate-600 via-teal-800 to-white">
					<Features
						header="Continuous integration"
						description="Highly efficient pipelines"
						features={ciFeatures}
						tier={3}
					/>
				</div>
			</main>
		</Layout>
	);
}
