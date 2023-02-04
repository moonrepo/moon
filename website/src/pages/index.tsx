import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Hero from '@site/src/components/Home/Hero';
import UsedBy from '@site/src/components/Home/UsedBy';
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

					<h2 className="mt-1 mb-3 text-white font-normal text-center text-2xl sm:text-3xl md:text-5xl">
						Next generation productivity tooling
					</h2>

					<p className="mm-0 text-white text-base text-center opacity-60 sm:text-lg md:text-xl md:max-w-xl">
						moonrepo is a better way to manage codebases, organize projects, orchestrate tasks, and
						scale your business.
					</p>
				</div>
			</div>

			<main className="bg-gradient-to-b from-slate-600 via-purple-600 to-white">
				<UsedBy />
			</main>
		</Layout>
	);
}
