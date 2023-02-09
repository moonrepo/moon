import React from 'react';
import Link from '@docusaurus/Link';
import { faRocketLaunch } from '@fortawesome/pro-duotone-svg-icons';
import Text from '@site/src/ui/typography/Text';
import LogoIcon from '../../../../static/brand/moon/icon.svg';
import TextVector from '../../../../static/brand/moon/text-vector.svg';
import YC from '../../../../static/img/logo-yc.svg';
import Icon from '../../../ui/iconography/Icon';
import DocLink from '../../../ui/typography/Link';
import CTA from '../../Home/CTA';
import HeroTerminal from './HeroTerminal';

// eslint-disable-next-line import/no-extraneous-dependencies
const { version } = require('@moonrepo/cli/package.json') as { version: string };

export default function Hero() {
	return (
		<div className="bg-gradient-to-b from-slate-900 to-slate-600 star-pattern">
			<div className="max-w-7xl mx-auto py-10 px-4 pb-6 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 flex flex-col md:flex-row">
				<div className="text-center md:text-left md:w-6/12">
					<h1 className="text-white flex justify-center md:justify-start items-end gap-2">
						<LogoIcon height={75} />
						<TextVector height={51} />
					</h1>

					<p className="mt-1 mb-0 text-base text-white sm:text-lg sm:max-w-xl sm:mx-auto md:text-xl md:mx-0 md:pr-4">
						A build system and repository management tool for the web ecosystem, written in Rust.
					</p>

					<p className="mt-1 text-white opacity-50 text-sm md:text-base md:pr-4">
						Supports JavaScript, TypeScript, Rust, Go, Ruby,{' '}
						<DocLink href="/docs#supported-languages" variant="muted">
							and more
						</DocLink>
						.
					</p>

					<div className="mt-3 flex justify-center md:justify-start">
						<div>
							<CTA href="/docs/install">
								Get started
								<Icon
									icon={faRocketLaunch}
									className="ml-1 md:ml-2 inline-block"
									style={{ maxWidth: 18 }}
								/>
							</CTA>
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

				<div className="mt-4 md:mt-0 md:w-6/12 flex flex-col flex-grow-0">
					<HeroTerminal />

					<div className="mt-2 flex justify-center items-start gap-1">
						<div>
							<Text className="text-white opacity-50" size="sm">
								Backed by
							</Text>
						</div>
						<div>
							<YC height={22} />
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
