/* eslint-disable sort-keys */

import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {
	faAperture,
	faArrowUpRightDots,
	faBellOn,
	faBoxAlt,
	faChartTreeMap,
	faCircleBolt,
	faCloudArrowUp,
	faDiagramProject,
	faDiagramSankey,
	faFingerprint,
	faGridDividers,
	faLayerGroup,
	faMerge,
	faMessageCode,
	faMicrochip,
	faRectangleBarcode,
	faShieldHalved,
	faSitemap,
	faSlidersUp,
	faToolbox,
	faUserSecret,
} from '@fortawesome/pro-duotone-svg-icons';
import AdditionalFeatures from '@site/src/components/Home/AdditionalFeatures';
import Features, { Feature } from '@site/src/components/Home/Features';
import Hero from '@site/src/components/Home/Hero';
import UsedBy from '@site/src/components/Home/UsedBy';
import Layout from '@theme/Layout';

const managementFeatures: Feature[] = [
	{
		title: 'Smart hashing',
		icon: faFingerprint,
		description:
			'Collects inputs from multiple sources to ensure builds are deterministic and reproducible.',
	},
	{
		title: 'Remote caching',
		icon: faCloudArrowUp,
		description: 'Persists builds, hashes, and caches between teammates and CI/CD environments.',
		status: 'experimental',
	},
	{
		title: 'Integrated toolchain',
		icon: faToolbox,
		description:
			'Automatically downloads and installs explicit versions of Node.js and other tools for consistency across the entire workspace or per project.',
	},
	{
		title: 'Multi-platform',
		icon: faMicrochip,
		description: 'Runs on common development platforms: Linux, macOS, and Windows.',
	},
];

const organizationFeatures: Feature[] = [
	{
		title: 'Project graph',
		icon: faDiagramProject,
		description: 'Generates a project graph for dependency and dependent relationships.',
	},
	{
		title: 'Code generation',
		icon: faLayerGroup,
		description: 'Easily scaffold new applications, libraries, tooling, and more!',
	},
	{
		title: 'Dependency workspaces',
		icon: faGridDividers,
		description:
			'Works alongside package manager workspaces so that projects have distinct dependency trees.',
	},
	{
		title: 'Ownership metadata',
		icon: faRectangleBarcode,
		description:
			'Declare an owner, maintainers, support channels, and more, for LDAP or another integration.',
	},
];

const orchestrationFeatures: Feature[] = [
	{
		title: 'Dependency graph',
		icon: faSitemap,
		description: 'Generates a dependency graph to increase performance and reduce workloads.',
	},
	{
		title: 'Action runner',
		icon: faMerge,
		description:
			'Executes actions in parallel and in order using a thread pool and our dependency graph.',
	},
	{
		title: 'Action distribution',
		icon: faDiagramSankey,
		description: 'Distributes actions across multiple machines to increase throughput.',
		status: 'coming-soon',
	},
	{
		title: 'Incremental builds',
		icon: faArrowUpRightDots,
		description:
			'With our smart hashing, only rebuild projects that have been touched since the last build.',
	},
];

const notificationFeatures: Feature[] = [
	{
		title: 'Flakiness detection',
		icon: faShieldHalved,
		description: 'Reduce flaky builds with automatic retries and passthrough settings.',
		status: 'experimental',
	},
	{
		title: 'Webhook events',
		icon: faMessageCode,
		description:
			'Receive a webhook for every event in the pipeline. Useful for metrics gathering and insights.',
		status: 'experimental',
	},
	{
		title: 'Terminal notifications',
		icon: faBellOn,
		description:
			'Receives notifications in your chosen terminal when builds are successful... or are not.',
		status: 'coming-soon',
	},
];

const additionalFeatures: Feature[] = [
	{
		title: 'Configuration & convention',
		icon: faSlidersUp,
		description: 'Use moon the way you want, but with some guard rails.',
	},
	{
		title: 'Scalability aware',
		icon: faCircleBolt,
		description: 'Engineered to scale and grow for codebases of any size.',
	},
	{
		title: 'Integrated packages',
		icon: faBoxAlt,
		description: (
			<>
				Enhance your pipeline with our{' '}
				<Link href="https://www.npmjs.com/org/moonrepo">@moonrepo</Link> npm packages.
			</>
		),
		status: 'in-development',
	},
	{
		title: 'Node.js inspection',
		icon: faUserSecret,
		description: 'Inspect and debug failing Node.js processes.',
		status: 'coming-soon',
	},
	{
		title: 'Build profiles',
		icon: faAperture,
		description: (
			<>
				Record <Link href="/docs/guides/profile">CPU and heap profiles</Link> that can be analyzed
				in Chrome.
			</>
		),
	},
	{
		title: 'Editor extensions',
		icon: faChartTreeMap,
		description: (
			<>
				Utilize moon extensions in your favorite editor, like{' '}
				<Link href="/docs/editors/vscode">Visual Studio Code</Link>.
			</>
		),
		status: 'new',
	},
];

const description =
	'Reduces build times and repository maintenance costs through high-quality developer tooling.';

export default function Home() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<Layout title={siteConfig.tagline} description={description}>
			<Hero />

			<main>
				<Features
					header="Management"
					description="Develop more, manage less"
					features={managementFeatures}
					tier={1}
				/>

				<Features
					header="Organization"
					description="Architect a repository to scale"
					features={organizationFeatures}
					tier={2}
					reversed
				/>

				<Features
					header="Orchestration"
					description="Offload heavy tasks"
					features={orchestrationFeatures}
					tier={3}
				/>

				<Features
					header="Notification"
					description="Monitor pipeline health"
					features={notificationFeatures}
					// columns={3}
					tier={4}
					reversed
				/>

				<AdditionalFeatures header="And many more features" features={additionalFeatures} />

				<UsedBy />
			</main>
		</Layout>
	);
}
