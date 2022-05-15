import React from 'react';
import { faDiscord, faGithub, faTwitter } from '@fortawesome/free-brands-svg-icons';
import Icon from '../../../ui/iconography/Icon';
// import Heading from '../../../ui/typography/Heading';
import Link from '../../../ui/typography/Link';
import Text from '../../../ui/typography/Text';

export default function FooterLayout({ style, links, logo, copyright }) {
	return (
		<footer className="bg-gray-100 dark:bg-slate-600" aria-labelledby="footer-heading">
			<h2 id="footer-heading" className="sr-only">
				Footer
			</h2>

			<div className="max-w-7xl mx-auto py-3 px-2 sm:px-3 md:py-4 md:px-4 lg:px-6">
				<div className="lg:grid lg:grid-cols-3 lg:gap-4">
					<div className="grid grid-cols-3 gap-4 lg:col-span-3">{links}</div>

					{/* <div className="mt-4 lg:mt-0">
						<Heading level={6} transform="uppercase">
							Subscribe for updates
						</Heading>

						<form className="mt-2 sm:flex sm:max-w-md">
							<label htmlFor="email" className="sr-only">
								Email address
							</label>

							<input
								type="email"
								name="email"
								id="email"
								autoComplete="email"
								required
								className="appearance-none outline-none min-w-0 w-full bg-white border border-transparent rounded-md px-1 py-1 sm:px-2 sm:py-2 text-base text-gray-800 placeholder-gray-600"
								placeholder="Email address"
							/>

							<div className="mt-1 rounded-md sm:mt-0 sm:ml-1 sm:flex-shrink-0">
								<button
									type="submit"
									className="w-full border border-transparent rounded-md px-2 py-1 sm:px-3 sm:py-2 flex items-center justify-center text-base font-bold text-white hover:text-white bg-blurple-400 hover:bg-blurple-500 dark:bg-purple-600 dark:hover:bg-purple-500 cursor-pointer"
								>
									Subscribe
								</button>
							</div>
						</form>
					</div> */}
				</div>

				<div className="mt-3 pt-3 md:mt-4 md:pt-4 border-0 border-t border-solid border-gray-200 dark:border-slate-400 flex items-center justify-between">
					<Text variant="muted" size="sm" as="div">
						{copyright}
					</Text>

					<div className="flex space-x-2">
						<Link href="https://github.com/moonrepo/moon">
							<span className="sr-only">GitHub</span>
							<Icon icon={faGithub} />
						</Link>

						<Link href="https://discord.gg/qCh9MEynv2">
							<span className="sr-only">Discord</span>
							<Icon icon={faDiscord} />
						</Link>

						<Link href="https://twitter.com/tothemoonrepo">
							<span className="sr-only">Twitter</span>
							<Icon icon={faTwitter} />
						</Link>
					</div>
				</div>
			</div>
		</footer>
	);
}
