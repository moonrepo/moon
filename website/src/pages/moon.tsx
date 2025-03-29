/* eslint-disable sort-keys */

import Link from '@docusaurus/Link';
import AdditionalFeatures from '@site/src/components/Products/AdditionalFeatures';
import Features, { Feature } from '@site/src/components/Products/Features';
import Hero from '@site/src/components/Products/Moon/Hero';
import Layout from '@theme/Layout';

const managementFeatures: Feature[] = [
	{
		title: 'Smart hashing',
		icon: 'material-symbols:fingerprint',
		description:
			'Collects inputs from multiple sources to ensure builds are deterministic and reproducible.',
	},
	{
		title: 'Remote caching',
		icon: 'material-symbols:cloud-upload',
		description: 'Persists builds, hashes, and caches between teammates and CI/CD environments.',
	},
	{
		title: 'Integrated toolchain',
		icon: 'material-symbols:service-toolbox',
		description:
			'Automatically downloads and installs explicit versions of languages and tools for consistency across the entire workspace or per project.',
	},
	{
		title: 'Multi-platform',
		icon: 'material-symbols:hive',
		description: 'Runs on common development platforms: Linux, macOS, and Windows.',
	},
];

const organizationFeatures: Feature[] = [
	{
		title: 'Project graph',
		icon: 'material-symbols:flowchart',
		description: 'Generates a project graph for dependency and dependent relationships.',
	},
	{
		title: 'Code generation',
		icon: 'material-symbols:stacks',
		description: 'Easily scaffold new applications, libraries, tooling, and more!',
	},
	{
		title: 'Task inheritance',
		icon: 'material-symbols:graph-4',
		description:
			'Define a task once to be inherited by all or scoped projects. Avoid defining the same task in every project.',
	},
	{
		title: 'Code ownership',
		icon: 'material-symbols:barcode-scanner',
		description: 'Declare owners, maintainers, support channels, and more. Generate CODEOWNERS.',
	},
];

const orchestrationFeatures: Feature[] = [
	{
		title: 'Dependency graph',
		icon: 'material-symbols:graph-3',
		description: 'Generates a dependency graph to increase performance and reduce workloads.',
	},
	{
		title: 'Action pipeline',
		icon: 'material-symbols:stacked-line-chart',
		description:
			'Executes actions in parallel and in order using a thread pool and our dependency graph.',
	},
	// {
	// 	title: 'Action distribution',
	// 	icon: 'material-symbols:arrow-upload-progress',
	// 	description: 'Distributes actions across multiple machines to increase throughput.',
	// 	status: 'coming-soon',
	// },
	{
		title: 'Advanced Git',
		icon: 'material-symbols:network-node',
		description: 'Supports advanced Git features like submodules, subtrees, and worktrees.',
	},
	{
		title: 'Incremental builds',
		icon: 'material-symbols:stacked-bar-chart',
		description:
			'With our smart hashing, only rebuild projects that have been touched since the last build.',
	},
];

const notificationFeatures: Feature[] = [
	{
		title: 'Flakiness detection',
		icon: 'material-symbols:shield-question',
		description: 'Reduce flaky builds with automatic retries and passthrough settings.',
	},
	{
		title: 'Webhook events',
		icon: 'material-symbols:webhook',
		description:
			'Receive a webhook for every event in the pipeline. Useful for metrics gathering and insights.',
		status: 'experimental',
	},
	{
		title: 'Terminal notifications',
		icon: 'material-symbols:deployed-code-alert',
		description:
			'Receives notifications in your chosen terminal when builds are successful... or are not.',
		status: 'coming-soon',
	},
	{
		title: 'Git hooks',
		icon: 'material-symbols:text-compare',
		description: 'Manage Git hooks to enforce workflows and requirements for contributors.',
	},
];

const additionalFeatures: Feature[] = [
	{
		title: 'Configuration & convention',
		icon: 'material-symbols:full-stacked-bar-chart',
		description: 'Use moon the way you want, but with some guard rails.',
	},
	{
		title: 'Scalability aware',
		icon: 'material-symbols:multiline-chart',
		description: 'Engineered to scale and grow for codebases of any size.',
	},
	{
		title: 'Integrated packages',
		icon: 'material-symbols:package-2',
		description: (
			<>
				Enhance your pipeline with our{' '}
				<Link href="https://www.npmjs.com/org/moonrepo">@moonrepo</Link> npm packages.
			</>
		),
		status: 'in-development',
	},
	{
		title: 'Process inspection',
		icon: 'material-symbols:frame-inspect',
		description: 'Inspect and debug failing task processes.',
		status: 'coming-soon',
	},
	{
		title: 'Build profiles',
		icon: 'material-symbols:settings-account-box',
		description: (
			<>
				Record <Link href="/docs/guides/profile">CPU and heap profiles</Link> that can be analyzed
				in Chrome.
			</>
		),
	},
	{
		title: 'Editor extensions',
		icon: 'material-symbols:power-plug',
		description: (
			<>
				Utilize moon extensions in your favorite editor, like{' '}
				<Link href="/docs/editors/vscode">Visual Studio Code</Link>.
			</>
		),
	},
];

export default function ProductMoon() {
	return (
		<Layout
			title="moon - A task runner and monorepo management tool for the web ecosystem, written in Rust"
			description="Reduces build times and repository maintenance costs through high-quality developer tooling."
		>
			<Hero />

			<main>
				<div className="bg-gradient-to-b from-slate-600 via-purple-600 to-white">
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
						description="Efficiently execute tasks"
						features={orchestrationFeatures}
						tier={3}
					/>

					<Features
						header="Notification"
						description="Monitor pipeline health"
						features={notificationFeatures}
						tier={4}
						reversed
					/>
				</div>

				<AdditionalFeatures header="And many more features" features={additionalFeatures} />
			</main>
		</Layout>
	);
}
