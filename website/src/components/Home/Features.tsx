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
	columns?: 3 | 4 | 5;
	tier: FeatureTier;
}

const backgroundGradients: Record<FeatureTier, string> = {
	1: 'from-slate-600 to-purple-700',
	2: 'from-purple-700 to-purple-200',
	3: 'from-purple-200 to-white',
	4: 'bg-white',
};

const headings: Record<FeatureTier, string> = {
	1: 'text-white',
	2: 'text-white',
	3: 'text-gray-900',
	4: 'text-gray-900',
};

const titles: Record<FeatureTier, string> = {
	1: 'text-purple-500',
	2: 'text-purple-300',
	3: 'text-pink-600',
	4: 'text-teal-600',
};

const cardIcons: Record<FeatureTier, string> = {
	1: 'text-purple-400',
	2: 'text-purple-700',
	3: 'text-pink-600',
	4: 'text-teal-600',
};

const cardForegrounds: Record<FeatureTier, string> = {
	1: 'text-purple-200',
	2: 'text-purple-800',
	3: 'text-gray-800',
	4: 'text-gray-700',
};

const cardBackgrounds: Record<FeatureTier, string> = {
	1: 'from-white/10 to-white/0',
	2: 'from-white/20 to-white/0',
	3: 'from-white/40 to-white/0',
	4: 'from-gray-100/30 to-white',
};

const cardHeadings: Record<FeatureTier, string> = {
	1: 'text-white',
	2: 'text-white',
	3: 'text-gray-900',
	4: 'text-gray-900',
};

const columnClasses = {
	3: 'sm:grid-cols-2 lg:grid-cols-3',
	4: 'sm:grid-cols-2 lg:grid-cols-4',
	5: 'sm:grid-cols-3 lg:grid-cols-5',
};

export default function Features({
	header,
	description,
	features,
	columns = 4,
	tier,
}: FeaturesProps) {
	return (
		<div className={cx('bg-gradient-to-b', backgroundGradients[tier])}>
			<div className="relative py-4 sm:py-5 lg:py-6">
				<div className="mx-auto max-w-md px-2 text-center sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<h2 className={cx('m-0 text-base font-semibold uppercase tracking-wider', titles[tier])}>
						{header}
					</h2>

					<Heading className={cx('mt-1', headings[tier])} level={2}>
						{description}
					</Heading>

					<div className="mt-4">
						<div className={cx('grid grid-cols-1 gap-4', columnClasses[columns])}>
							{features.map((feature) => (
								<div key={feature.title} className="pt-6">
									<div
										className={cx(
											'flow-root rounded-lg px-2 pb-3 bg-gradient-to-b',
											cardBackgrounds[tier],
										)}
									>
										<div className="-mt-3">
											<div>
												<Icon
													icon={feature.icon}
													className={cx(
														'inline-flex items-center justify-center text-5xl',
														cardIcons[tier],
													)}
												/>
											</div>

											<Heading className={cx('mt-2', cardHeadings[tier])} level={4}>
												{feature.title}
											</Heading>

											{feature.status && (
												<p>
													<FeatureStatus status={feature.status} />
												</p>
											)}

											<Text className={cx('mt-2', cardForegrounds[tier])}>
												{feature.description}
											</Text>
										</div>
									</div>
								</div>
							))}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
