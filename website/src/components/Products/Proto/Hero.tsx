/* eslint-disable promise/prefer-await-to-then */

import React, { useEffect, useState } from 'react';
import Link from '@docusaurus/Link';
import LogoIcon from '../../../../static/brand/proto/icon.svg';
import TextVector from '../../../../static/brand/proto/text-vector.svg';
import YC from '../../../../static/img/logo-yc.svg';
import Icon from '../../../ui/iconography/Icon';
import Text from '../../../ui/typography/Text';
import CTA from '../../Home/CTA';
import HeroIcon from '../HeroIcon';
import HeroTerminal from './HeroTerminal';

// A lightspeed and pluggable toolchain manager for languages and their dependency managers.

export default function Hero() {
	const [version, setVersion] = useState('?.?.?');

	useEffect(() => {
		void fetch('https://raw.githubusercontent.com/moonrepo/proto/master/version')
			.then((res) => res.text())
			.then((text) => void setVersion(text.trim()))
			.catch(console.error);
	}, []);

	return (
		<div className="bg-gradient-to-b from-slate-900 to-slate-600 star-pattern">
			<div className="max-w-7xl mx-auto py-10 px-4 pb-6 sm:py-12 sm:px-6 md:py-14 lg:py-16 lg:px-8 xl:py-20 flex flex-col md:flex-row">
				<div className="text-center md:text-left md:w-6/12">
					<HeroIcon
						icon={<LogoIcon height={75} style={{ marginTop: 5 }} />}
						text={<TextVector height={90} />}
					/>

					<p className="mt-1 mb-0 text-base text-white sm:text-lg sm:max-w-xl sm:mx-auto md:text-xl md:mx-0 md:w-[80%]">
						A version manager for all your favorite languages and tools. A unified toolchain.
					</p>

					<p className="mt-1 text-white opacity-50 text-sm md:text-base md:pr-4">
						Supports Bun, Deno, Node, Python, Rust, Go, and 800+ more.
					</p>

					<div className="mt-3 flex justify-center md:justify-start">
						<div>
							<CTA href="/docs/proto/install" color="bg-pink-600">
								Get started
								<Icon
									icon="material-symbols:wand-stars"
									className="ml-1 md:ml-2 inline-block rotate-180"
									style={{ maxWidth: 18 }}
								/>
							</CTA>
						</div>

						<div className="ml-1 sm:ml-2 lg:ml-3">
							<Link
								href="/docs/proto"
								className="w-full flex items-center justify-center px-2 py-1 sm:px-3 sm:py-2 text-base font-bold rounded-md text-white hover:text-white bg-white/5 hover:scale-110 md:text-lg group transition-transform"
							>
								v{version}
							</Link>
						</div>
					</div>
				</div>

				<div className="mt-4 md:mt-0 md:w-6/12 flex flex-col flex-grow-0">
					<HeroTerminal />

					<div className="mt-2 flex justify-center items-start gap-1">
						<div>
							<Text className="text-white opacity-50 m-0" size="sm">
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
