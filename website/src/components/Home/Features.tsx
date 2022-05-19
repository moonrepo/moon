import React from 'react';
import cx from 'clsx';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Icon from '../../ui/iconography/Icon';
import Heading from '../../ui/typography/Heading';
import Text from '../../ui/typography/Text';
import FeatureStatus, { StatusType } from './FeatureStatus';

export type FeatureTier = 1 | 2 | 3 | 4;

export interface Feature {
	title: string;
	icon: IconDefinition;
	description: React.ReactNode;
	status?: StatusType;
}

export interface FeaturesProps {
	header: string;
	description: string;
	features: Feature[];
	reversed?: boolean;
	tier: FeatureTier;
}

const backgroundGradients: Record<FeatureTier, string> = {
	1: 'from-slate-600 to-purple-700',
	2: 'from-purple-700 to-purple-200',
	3: 'from-purple-200 to-white',
	4: 'bg-white',
};

const cardGradients: Record<FeatureTier, string> = {
	1: 'from-purple-600 to-blurple-600',
	2: 'from-pink-400 to-red-400',
	3: 'from-teal-400 to-slate-100',
	4: 'from-yellow-300 to-green-600',
};

const iconColors: Record<FeatureTier, string> = {
	1: 'text-blurple-300',
	2: 'text-pink-500',
	3: 'text-teal-600',
	4: 'text-green-600',
};

export default function Features({ header, description, features, reversed, tier }: FeaturesProps) {
	return (
		<div className={cx('bg-gradient-to-b', backgroundGradients[tier])}>
			<div className="relative py-4 sm:py-5 lg:py-6">
				<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<div
						className={cx('md:flex items-center justify-between', reversed && 'flex-row-reverse')}
					>
						<aside
							className={cx(
								'md:w-1/3 mx-2 md:mx-0 p-2 sm:p-3 md:p-4 drop-shadow z-0 rounded-t-lg text-center',
								reversed
									? 'md:rounded-tl-none md:rounded-tr-lg md:rounded-bl-none md:rounded-br-lg bg-gradient-to-bl md:pl-0 md:text-left'
									: 'md:rounded-tl-lg md:rounded-tr-none md:rounded-bl-lg md:rounded-br-none bg-gradient-to-br md:pr-0 md:text-right',
								cardGradients[tier],
							)}
						>
							<h2
								className={cx(
									'm-0 px-1 py-0.5 inline-block text-base font-semibold uppercase tracking-wider text-white bg-black/20 rounded',
									reversed
										? 'md:rounded-tl-none md:rounded-bl-none md:pl-4'
										: 'md:rounded-tr-none md:rounded-br-none md:pr-4',
								)}
							>
								{header}
							</h2>

							<Heading
								className={cx('mt-2 text-white', reversed ? 'md:ml-4' : 'md:mr-4')}
								level={2}
							>
								{description}
							</Heading>
						</aside>

						<section className="md:w-2/3 bg-white rounded-lg p-2 md:p-3 lg:p-4 drop-shadow z-10">
							<ul className="m-0 p-0 list-none grid grid-cols-1 sm:grid-cols-2 gap-2 md:gap-3 lg:gap-4">
								{features.map((feature) => (
									<li key={feature.title} className="flex">
										<div className="pt-1 w-9 shrink-0 grow-0">
											<Icon
												icon={feature.icon}
												className={cx('text-5xl justify-center flex', iconColors[tier])}
												style={{ maxWidth: 54 }}
											/>
										</div>

										<div className="ml-1">
											<Heading level={4} className="text-gray-900">
												{feature.title}
											</Heading>

											{feature.status && (
												<p className="m-0">
													<FeatureStatus status={feature.status} />
												</p>
											)}

											<Text className="mt-1" variant="muted">
												{feature.description}
											</Text>
										</div>
									</li>
								))}
							</ul>
						</section>
					</div>
				</div>
			</div>
		</div>
	);
}
