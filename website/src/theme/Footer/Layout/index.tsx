import React from 'react';
import { faDiscord, faGithub, faTwitter } from '@fortawesome/free-brands-svg-icons';
import Icon from '../../../ui/iconography/Icon';
import Heading from '../../../ui/typography/Heading';
import Link from '../../../ui/typography/Link';
import Text from '../../../ui/typography/Text';
import ContactForm from './ContactForm';

export interface FooterLayoutProps {
	copyright: NonNullable<React.ReactNode>;
	links: React.ReactNode;
}

export default function FooterLayout({ links, copyright }: FooterLayoutProps) {
	return (
		<footer className="bg-gray-100 dark:bg-slate-600" aria-labelledby="footer-heading">
			<h2 id="footer-heading" className="sr-only">
				Footer
			</h2>

			<div className="max-w-7xl mx-auto py-3 px-2 sm:px-3 md:py-4 md:px-4 lg:px-6">
				<div className="lg:grid lg:grid-cols-5 lg:gap-3">
					<div className="grid grid-cols-3 gap-3 lg:col-span-3">{links}</div>

					<div className="mt-4 lg:mt-0 col-span-2">
						<Heading level={6} transform="uppercase">
							Contact us
						</Heading>

						<ContactForm />
					</div>
				</div>

				<div className="mt-3 pt-3 md:mt-4 md:pt-4 border-0 border-t border-solid border-gray-200 dark:border-slate-400 flex items-center justify-between">
					<Text variant="muted" size="sm" as="div">
						{copyright}
					</Text>

					<div className="flex space-x-2">
						<Link href="https://github.com/moonrepo">
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
