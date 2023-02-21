import React from 'react';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import { faDiagramProject, faDiagramSankey, faToolbox } from '@fortawesome/pro-duotone-svg-icons';
import ProductSection from '@site/src/components/Home/ProductSection';
import UsedBy from '@site/src/components/Home/UsedBy';
import MoonbaseScreenshots from '@site/src/components/Products/Moonbase/Screenshots';
import Link from '@site/src/ui/typography/Link';
import Layout from '@theme/Layout';
import TextVector from '../../static/brand/moonrepo/text-vector.svg';

export default function Home() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<Layout
			title={siteConfig.tagline}
			description="From build to deploy, moonrepo is a better way to manage codebases, save developer time, and boost your business."
		>
			<div className="bg-gradient-to-b from-slate-900 to-slate-600">
				<div className="max-w-7xl mx-auto py-10 px-4 pb-6 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 flex flex-col justify-center items-center">
					<h1 className="text-white">
						<TextVector height={65} />
					</h1>

					<h2 className="mb-3 text-white font-medium text-center text-3xl sm:text-4xl md:text-5xl">
						New era of productivity tooling
					</h2>

					<p className="mm-0 text-white text-md text-center opacity-60 px-4 sm:text-lg md:text-xl md:max-w-3xl">
						From build to deploy, moonrepo is a better way to manage codebases, save developer time,
						and boost your business.
					</p>
				</div>
			</div>

			<main className="bg-gradient-to-b from-slate-600 via-blurple-600 to-white">
				<ProductSection
					color="text-blurple-400"
					suptitle="A system for a solid foundation"
					title="Supercharge your codebase"
					logo={<img src="/brand/moon/icon.svg" height={75} className="block" />}
					description={
						<>
							For repositories with multiple projects, any number of languages, and team members
							constantly pushing changes, our task runner{' '}
							<Link href="/moon" size="lg">
								moon
							</Link>{' '}
							will help simplify the experience of working in and maintaining your codebase.
						</>
					}
					cta={{
						children: 'Learn more about moon',
						color: 'bg-blurple-400',
						href: '/moon',
					}}
					items={[
						{
							description:
								'Never run the same task twice. With our smart hashing, robust caching, and efficient task execution, moon will avoid unnecessary work.',
							icon: faDiagramSankey,
							title: 'Efficient task orchestation',
						},
						{
							description:
								'Neatly organize your codebase, declare ownership information, and simplify project discovery.',
							icon: faDiagramProject,
							title: 'Better project organization',
						},
						{
							description:
								'With our integrated toolchain, the exact tooling version will be used, ensuring a deterministic environment across machines.',
							icon: faToolbox,
							title: 'Integrated development environment',
						},
					]}
				/>

				<ProductSection
					reversed
					stretched
					color="text-purple-600"
					suptitle="A service to expand to the cloud"
					title="Accelerate your pipelines"
					logo={<img src="/brand/moonbase/icon.svg" height={75} className="block" />}
					description={
						<>
							With our hosted service{' '}
							<Link href="/moon" size="lg">
								moonbase
							</Link>
							, easily cache build artifacts to reduce CI times, gain insight into your CI
							pipelines, track the health of your repositories, and overall cut costs.
						</>
					}
					cta={{
						children: 'Learn more about moonbase',
						color: 'bg-purple-600',
						href: '/moonbase',
					}}
				>
					<div className="relative sm:pb-8 h-full">
						<MoonbaseScreenshots />
					</div>
				</ProductSection>
			</main>

			<UsedBy />
		</Layout>
	);
}
