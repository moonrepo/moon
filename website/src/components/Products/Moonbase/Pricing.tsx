import React from 'react';
import { faCheck } from '@fortawesome/pro-regular-svg-icons';
import Button, { ButtonProps } from '@site/src/ui/Button';
import Icon from '@site/src/ui/iconography/Icon';
import Heading from '@site/src/ui/typography/Heading';
import Link from '@site/src/ui/typography/Link';
import Text from '@site/src/ui/typography/Text';

interface TierProps {
	children: React.ReactNode;
	cta: ButtonProps;
	items: { label: string; monthly?: boolean; tooltip?: string }[];
	title: string;
}

function Tier({ children, cta, items, title }: TierProps) {
	return (
		<div>
			<div className="text-center mb-3">
				<Heading level={4} className="mb-1">
					{title}
				</Heading>

				{children}
			</div>

			<div className="bg-gray-50 rounded p-4 lg:h-[260px]">
				<ul className="flex flex-col gap-2 p-0 m-0">
					{items.map((item) => (
						<li key={item.label} className="list-none relative pl-4">
							<abbr title={item.tooltip}>{item.label}</abbr>

							{item.monthly && <span className="text-gray-700 inline-block ml-0.5">/ month</span>}

							<div className="absolute top-0 left-0 text-blurple-400">
								<Icon icon={faCheck} />
							</div>
						</li>
					))}
				</ul>
			</div>

			<div className="flex justify-center mt-2">
				<Button {...cta} />
			</div>
		</div>
	);
}

export default function Pricing() {
	return (
		<div id="pricing" className="relative py-4 sm:py-5 lg:py-6 bg-white text-slate-900">
			<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
				<div className="text-center mb-4">
					<Heading level={2}>Pricing</Heading>

					<p className="mt-1">
						Use moonbase for free for small, personal, or open source projects.
						<br />
						Upgrade for more features or for larger teams.
					</p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-3 gap-3">
					<div>
						<Tier
							title="Start"
							items={[
								{
									label: '1 organization member',
									tooltip: 'Maximum number of members per organization, including the owner.',
								},
								{ label: '5 repositories', tooltip: 'Maximum number per organization.' },
								// {
								// 	label: '25 projects',
								// 	tooltip:
								// 		'Maximum number of projects aggregated into the registry, across all repositories.',
								// },
								{ label: '100 CI runs', monthly: true, tooltip: 'Across all repositories.' },
								{ label: '1GB cloud storage', tooltip: 'Across all repositories.' },
							]}
							cta={{ href: 'https://moonrepo.app', label: 'Get started' }}
						>
							<Heading level={1}>Free</Heading>
						</Tier>
					</div>
					<div>
						<Tier
							title="Scale"
							items={[
								{ label: 'Priority support', tooltip: 'Get faster replies to support questions.' },
								{
									label: '5 free repositories',
									tooltip: 'Increased cost for additional repositories.',
								},
								// {
								// 	label: 'Unlimited projects',
								// 	tooltip:
								// 		'Maximum number of projects aggregated into the registry, across all repositories.',
								// },
								{ label: '1,000 CI runs', monthly: true, tooltip: 'Across all repositories.' },
								{
									label: '10GB cloud storage',
									tooltip: 'Across all repositories.',
								},
								{
									label: 'Unlocked organization settings',
								},
							]}
							cta={{ href: 'https://moonrepo.app/upgrade', label: 'Upgrade now' }}
						>
							<div className="flex justify-center gap-2">
								<div>
									<Heading level={1}>$5</Heading>
								</div>
								<div className="text-left">
									per member + repo
									<Text variant="muted">monthly</Text>
								</div>
							</div>
						</Tier>
					</div>
					<div>
						<Tier
							title="Grow"
							items={[
								{
									label: 'Enterprise support',
									tooltip: 'Get instant replies to support questions.',
								},
								{
									label: '15 free repositories',
									tooltip: 'Increased cost for additional repositories.',
								},
								// {
								// 	label: 'On-premise solution',
								// 	tooltip: 'Host moonbase within your infrastructure.',
								// },
								// {
								// 	label: 'Unlimited projects',
								// 	tooltip:
								// 		'Maximum number of projects aggregated into the registry, across all repositories.',
								// },
								{ label: 'Unlimited CI runs', monthly: true, tooltip: 'Across all repositories.' },
								{
									label: 'Unlimited cloud storage',
									tooltip: 'Across all repositories.',
								},
								{
									label: '+ previous tier',
								},
							]}
							cta={{ disabled: true, label: 'Coming soon' }}
						>
							{/* <div className="flex justify-center gap-2">
								<div>
									<Heading level={1}>$12</Heading>
								</div>
								<div className="text-left">
									per member / repo
									<Text variant="muted">monthly</Text>
								</div>
							</div> */}

							<Heading level={1}>Soon</Heading>
						</Tier>
					</div>
				</div>

				<div className="mt-6 text-center">
					<Text size="sm" variant="muted">
						<Link size="sm" href="https://moonrepo.app/terms">
							Terms of Service
						</Link>
						{' · '}
						<Link size="sm" href="https://moonrepo.app/privacy">
							Privacy Policy
						</Link>
					</Text>

					<Text size="sm" variant="muted">
						Prices and limits subject to change before release!
					</Text>
				</div>
			</div>
		</div>
	);
}
