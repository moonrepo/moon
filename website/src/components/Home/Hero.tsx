import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import { faRocketLaunch } from '@fortawesome/pro-regular-svg-icons';
import Icon from '../../ui/iconography/Icon';

export default function Hero() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<div className="bg-gradient-to-b from-slate-900 to-slate-600 star-pattern">
			<div className="max-w-7xl mx-auto py-10 px-4 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 text-center lg:text-left">
				<h1 className="text-6xl tracking-tight font-extrabold text-purple-600">
					{siteConfig.title}
				</h1>

				<p className="mt-1 mb-0 text-base text-white sm:text-lg sm:max-w-xl sm:mx-auto md:text-xl lg:mx-0">
					{siteConfig.tagline}
					<span className="opacity-50">, powered by Rust.</span>
				</p>

				<div className="mt-3 sm:mt-3 sm:flex sm:justify-center lg:justify-start">
					<div>
						<Link
							href="/docs/install"
							className="w-full flex items-center justify-center px-2 py-1 text-base font-bold rounded-md text-white hover:text-white bg-purple-600 hover:scale-110 sm:px-3 sm:py-2 md:text-lg group transition-transform"
						>
							Get started
							<Icon
								icon={faRocketLaunch}
								className="ml-1 md:ml-2 inline-block opacity-75 group-hover:opacity-100"
							/>
						</Link>
					</div>

					<div className="mt-1 ml-0 sm:mt-0 sm:ml-2 lg:ml-3">
						<Link
							href="https://www.npmjs.com/package/@moonrepo/cli"
							className="w-full flex items-center justify-center px-2 py-1 text-base font-bold rounded-md text-white hover:text-white bg-white/5 hover:scale-110 sm:px-3 sm:py-2 md:text-lg group transition-transform"
						>
							<span className="opacity-50">v</span>1.2.3
						</Link>
					</div>
				</div>
			</div>
		</div>
	);
}
