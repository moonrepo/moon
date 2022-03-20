import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Features, { FeaturesProps } from '@site/src/components/Home/Features';
import {
	faFingerprint,
	faCloudArrowUp,
	faMicrochip,
	faToolbox,
	faDiagramProject,
	faRectangleBarcode,
	faBarcode,
	faGridHorizontal,
	faGridDividers,
} from '@fortawesome/pro-duotone-svg-icons';
import Layout from '@theme/Layout';
import styles from './index.module.css';

// - Configurable							sliders-up
// - Extensible?							puzzle
// - Scalable									circle-bolt

// Orchestration
// - Dependency graph					sitemap
// - Task runner							merge
// - Task distribution				diagram-sankey
// - Parallel execution				arrows-turn-right
// - Incremental builds				arrow-up-right-dots

// Notification
// - Flakiness detection			shield-halved
// - Webhooks and events			message-code
// - Terminal notifications		bell-on

// Development
// - Node.js inspector integration	user-secret
// - Chrome profiling								aperture
// - Editor extensions						chart-tree-map

const managementFeatures: FeaturesProps['features'] = [
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
	},
	{
		title: 'Integrated toolchain',
		icon: faToolbox,
		description:
			'Automatically downloads and installs explicit versions of Node.js and other tools for consistency.',
	},
	{
		title: 'Multi-platform',
		icon: faMicrochip,
		description: 'Runs on common development platforms: Linux, MacOS, and Windows.',
	},
];

const organizationFeatures: FeaturesProps['features'] = [
	{
		title: 'Project graph',
		icon: faDiagramProject,
		description: 'Generates a project graph to increase performance and reduce workloads.',
	},
	{
		title: 'Project boundaries',
		icon: faGridHorizontal,
		description: 'Enforces boundaries to eliminate cycles and reduce indirection.',
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
			'Declare an owner, maintainers, support channels, and more, via LDAP or another integration.',
	},
];

function HomepageHeader() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<header className={clsx('hero hero--primary', styles.heroBanner)}>
			<div className="container">
				<h1 className="hero__title">{siteConfig.title}</h1>
				<p className="hero__subtitle">{siteConfig.tagline}</p>
				<div className={styles.buttons}>
					<Link className="button button--secondary button--lg" to="/docs/intro">
						Docusaurus Tutorial - 5min ⏱️
					</Link>
				</div>
			</div>
		</header>
	);
}

export default function Home() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<Layout
			title={`Hello from ${siteConfig.title}`}
			description="Description will go into a meta tag in <head />"
		>
			<HomepageHeader />

			<main>
				<Features
					header="Management"
					description="Automates the complexity away"
					features={managementFeatures}
				/>

				<Features
					header="Organization"
					description="Automates the complexity away"
					features={organizationFeatures}
				/>
			</main>
		</Layout>
	);
}
