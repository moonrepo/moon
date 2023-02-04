import React from 'react';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import { faDiagramProject, faDiagramSankey, faToolbox } from '@fortawesome/pro-duotone-svg-icons';
import CTA from '@site/src/components/Home/CTA';
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

					<h2 className="mt-1 mb-3 text-white font-normal text-center text-3xl sm:text-4xl md:text-5xl">
						Next generation productivity tooling
					</h2>

					<p className="mm-0 text-white text-md text-center opacity-60 sm:text-lg md:text-xl md:max-w-2xl">
						From develop to deploy, moonrepo is a better way to manage codebases, organize projects,
						and scale your business.
					</p>
				</div>
			</div>

			<main className="bg-gradient-to-b from-slate-600 via-purple-600 to-white">
				<FeatureSection
					color="text-blurple-400"
					suptitle="Everything you need"
					title="Supercharge your codebase"
					description={
						<>
							Whether your a single project repository, a multi-project repository, or a monolithic,
							our build system{' '}
							<Link href="/moon" size="lg">
								moon
							</Link>{' '}
							will help your codebase grow.
						</>
					}
					items={[
						{
							description:
								'Neatly organize your codebase, declare ownership information, and simplify project discovery.',
							icon: faDiagramProject,
							title: 'Better project organization',
						},
						{
							description:
								'Never run the same task twice. With our smart hashing, robust caching, and efficient task execution, moon will avoid unnecessary work.',
							icon: faDiagramSankey,
							title: 'Efficient task orchestation',
						},
						{
							description:
								'With our integrated toolchain, the exact version of languages will be used, ensuring a deterministic environment.',
							icon: faToolbox,
							title: 'Integrated development environment',
						},
					]}
				>
					<CTA href="/moon">Learn more about moon</CTA>
				</FeatureSection>

				<UsedBy />
			</main>
		</Layout>
	);
}
