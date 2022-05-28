import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import { faRocketLaunch } from '@fortawesome/pro-duotone-svg-icons';
import Icon from '../../ui/iconography/Icon';
import HeroTerminal from './HeroTerminal';

// eslint-disable-next-line import/no-extraneous-dependencies
const { version } = require('@moonrepo/cli/package.json') as { version: string };

export default function Hero() {
	const { siteConfig } = useDocusaurusContext();

	return (
		<div className="bg-gradient-to-b from-slate-900 to-slate-600 star-pattern">
			<div className="max-w-7xl mx-auto py-10 px-4 pb-6 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 flex flex-col md:flex-row">
				<div className="text-center lg:text-left md:w-6/12">
					<h1 className="text-6xl tracking-tight font-extrabold text-purple-600">
						<img
							src="/img/logo-hero.svg"
							alt={siteConfig.title}
							title={siteConfig.title}
							width={300}
						/>
					</h1>

					<p className="mt-1 mb-0 text-base text-white sm:text-lg sm:max-w-xl sm:mx-auto md:text-xl lg:mx-0">
						{siteConfig.tagline}
						<span className="opacity-50">
							,<br /> powered by Rust.
						</span>
					</p>

					<div className="mt-3 flex justify-center lg:justify-start">
						<div>
							<Link
								href="/docs/install"
								className="w-full flex items-center justify-center px-2 py-1 sm:px-3 sm:py-2 text-base font-bold rounded-md text-white hover:text-white bg-purple-600 hover:scale-110 md:text-lg transition-transform"
							>
								Get started
								<Icon
									icon={faRocketLaunch}
									className="ml-1 md:ml-2 inline-block"
									style={{ maxWidth: 18 }}
								/>
							</Link>
						</div>

						<div className="ml-1 sm:ml-2 lg:ml-3">
							<Link
								href="https://www.npmjs.com/package/@moonrepo/cli"
								className="w-full flex items-center justify-center px-2 py-1 sm:px-3 sm:py-2 text-base font-bold rounded-md text-white hover:text-white bg-white/5 hover:scale-110 md:text-lg group transition-transform"
							>
								<span className="opacity-50">v</span>
								{version}
							</Link>
						</div>
					</div>
				</div>

				<div className="mt-4 md:mt-0 md:w-6/12 flex flex-grow-0">
					<HeroTerminal />
				</div>
			</div>
		</div>
	);
}
