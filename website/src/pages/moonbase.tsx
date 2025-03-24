/* eslint-disable sort-keys */

import Features, { Feature } from '@site/src/components/Products/Features';
import Hero from '@site/src/components/Products/Moonbase/Hero';
// import Pricing from '@site/src/components/Products/Moonbase/Pricing';
import Layout from '@theme/Layout';

const ciFeatures: Feature[] = [
	{
		title: 'Artifact caching',
		icon: 'material-symbols:cloud-upload',
		description: 'Cache build artifacts between CI runs to reduce job times and overall costs.',
	},
	// {
	// 	title: 'Run history',
	// 	icon: faTimeline,
	// 	description:
	// 		'Track CI runs to detect flakiness, regressions, and time spent on task execution.',
	// },
	// {
	// 	title: 'Remote distribution',
	// 	icon: faDiagramSankey,
	// 	description: 'Distribute task execution across multiple remote agents to increase throughput.',
	// 	status: 'coming-soon',
	// },
];

// const ownershipFeatures: Feature[] = [
// 	{
// 		title: 'Project registry',
// 		icon: faSquareKanban,
// 		description:
// 			'An aggregated registry of all projects, across all repositories, within an organization.',
// 		status: 'in-development',
// 	},
// 	{
// 		title: 'Code owners',
// 		icon: faMessageCode,
// 		description:
// 			'A granular breakdown of which team or developer owns a portion of code within each project.',
// 		status: 'coming-soon',
// 	},
// 	{
// 		title: 'Health score',
// 		icon: faNotesMedical,
// 		description: 'Monitor the health of projects and avoid tech debt.',
// 		status: 'coming-soon',
// 	},
// ];

export default function ProductMoonbase() {
	return (
		<Layout
			title="moonbase - A service for monitoring codebases and scaling CI pipelines"
			description="Reduces job times, tracks CI jobs, manages project/code ownerships, and more."
		>
			<Hero />

			<main>
				<div className="bg-gradient-to-b from-slate-600 via-teal-800 to-white">
					<Features
						header="Continuous integration"
						description="Efficient pipelines"
						features={ciFeatures}
						tier={3}
					/>

					{/* <Features
						header="Ownership"
						description="Everything in one place"
						features={ownershipFeatures}
						tier={1}
						reversed
					/> */}
				</div>

				{/* <Pricing /> */}
			</main>
		</Layout>
	);
}
