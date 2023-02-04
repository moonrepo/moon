import React from 'react';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import { faDiagramProject, faDiagramSankey, faToolbox } from '@fortawesome/pro-duotone-svg-icons';
import FeatureSection from '@site/src/components/Home/FeatureSection';
import UsedBy from '@site/src/components/Home/UsedBy';
import Link from '@site/src/ui/typography/Link';
import Layout from '@theme/Layout';

const description =
	'Reduces build times and repository maintenance costs through high-quality developer tooling.';

export default function Home() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<Layout title={siteConfig.tagline} description={description}>
			<div className="bg-gradient-to-b from-slate-900 to-slate-600 star-pattern">
				<div className="max-w-7xl mx-auto py-10 px-4 pb-6 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 flex flex-col justify-center items-center">
					<h1 className="text-2xl tracking-tight font-extrabold text-purple-600">
						<img
							src="/img/logo-hero.svg"
							alt={siteConfig.title}
							title={siteConfig.title}
							width={200}
						/>
					</h1>

					<h2 className="mt-1 mb-3 text-white font-medium text-center text-3xl sm:text-4xl md:text-5xl">
						New era of productivity tooling
					</h2>

					<p className="mm-0 text-white text-md text-center opacity-60 sm:text-lg md:text-xl md:max-w-2xl">
						From develop to deploy, moonrepo is a better way to manage codebases, save costs, and
						scale your business.
					</p>
				</div>
			</div>

			<main className="bg-gradient-to-b from-slate-600 via-purple-600 to-white">
				<FeatureSection
					color="text-purple-600"
					suptitle="Build a solid foundation"
					title="Supercharge your codebase"
					description={
						<>
							Regardless of how many projects or languages are in your repository, or how many team
							members are pushing changes, our build system{' '}
							<Link href="/moon" size="lg">
								moon
							</Link>{' '}
							will help maintain your codebase.
							{/* <>
							Whether your repository is one project or many projects, or composed of multiple
							languages, our build system{' '}
							<Link href="/moon" size="lg">
								moon
							</Link>{' '}
							will help your codebase grow.
					</> */}
						</>
					}
					cta={{ children: 'Learn more about moon', color: 'bg-purple-600', href: '/moon' }}
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
								'With our integrated toolchain, the exact version of languages will be used, ensuring a deterministic environment across machines.',
							icon: faToolbox,
							title: 'Integrated development environment',
						},
					]}
				/>

				<FeatureSection
					reversed
					stretched
					color="text-blurple-400"
					suptitle="Expand to the cloud"
					title="Scale your pipelines"
					description={
						<>
							With our hosted service{' '}
							<Link href="/moon" size="lg">
								moonbase
							</Link>
							, easily cache artifacts to reduce CI times, gain insight into your CI pipelines,
							track the health of your repositories, and overall cut costs.
						</>
					}
					cta={{
						children: 'Learn more about moonbase',
						color: 'bg-blurple-400',
						href: '/moonbase',
					}}
				>
					<div className="relative pb-8 h-full">
						<div className="overflow-hidden rounded-lg w-[65%]  bg-[#000e19] p-1">
							<img src="/img/home/org.png" alt="moonbase - organization view" className="block" />
						</div>

						<div className="overflow-hidden rounded-lg w-[65%] bg-[#000e19] p-1 absolute bottom-0 right-0 z-10">
							<img src="/img/home/repo.png" alt="moonbase - repository view" className="block" />
						</div>
					</div>
				</FeatureSection>
			</main>

			<UsedBy />
		</Layout>
	);
}
