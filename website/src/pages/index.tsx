import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import Layout from '@theme/Layout';
import styles from './index.module.css';

// - Configurable
// - Extensible?
// - Scalable

// Management
// - Smart hashing
// - Remote caching
// - Multi-platform
// - Integrated toolchain

// Organization
// - Project graph
// - Project boundaries
// - Package workspaces
// - Ownership metadata

// Orchestration
// - Dependency graph
// - Task runner
// - Task distribution
// - Parallel execution
// - Deterministic builds
// - Incremental builds

// Notification
// - Flakiness detection
// - Webhooks and events
// - Terminal notifications

// Development
// - Node.js inspector integration
// - Chrome profiling
// - Editor extensions

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
				<HomepageFeatures />
			</main>
		</Layout>
	);
}
