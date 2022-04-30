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
					<div className={cx('flex items-center justify-between', reversed && 'flex-row-reverse')}>
						<aside
							className={cx(
								'w-1/3 p-4 drop-shadow z-0',
								reversed
									? 'rounded-tr-lg rounded-br-lg bg-gradient-to-bl pl-0'
									: 'text-right rounded-tl-lg rounded-bl-lg bg-gradient-to-br pr-0',
								cardGradients[tier],
							)}
						>
							<h2
								className={cx(
									'm-0 px-1 py-0.5 inline-block text-base font-semibold uppercase tracking-wider text-white bg-black/20',
									reversed
										? 'rounded-tr-lg rounded-br-lg pl-4'
										: 'rounded-tl-lg rounded-bl-lg pr-4',
								)}
							>
								{header}
							</h2>

							<Heading className={cx('mt-2 text-white', reversed ? 'ml-4' : 'mr-4')} level={2}>
								{description}
							</Heading>
						</aside>

						<section className="w-2/3 bg-white rounded-lg p-4 drop-shadow z-10">
							<ul className="m-0 p-0 list-none grid grid-cols-2 gap-4">
								{features.map((feature) => (
									<li key={feature.title} className="flex">
										<Icon
											icon={feature.icon}
											className={cx(
												'pt-1 w-9 text-5xl shrink-0 grow-0 justify-center flex',
												iconColors[tier],
											)}
										/>

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
