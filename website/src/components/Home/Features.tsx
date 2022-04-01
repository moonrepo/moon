import React from 'react';
import cx from 'clsx';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Heading from '../Heading';
import Icon from '../Icon';
import Text from '../Text';
import FeatureStatus, { StatusType } from './FeatureStatus';

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
}

const columnClasses = {
	3: 'sm:grid-cols-2 lg:grid-cols-3',
	4: 'sm:grid-cols-2 lg:grid-cols-4',
	5: 'sm:grid-cols-3 lg:grid-cols-5',
};

export default function Features({ header, description, features, columns = 4 }: FeaturesProps) {
	return (
		<div className="bg-white">
			<div className="relative py-5 sm:py-6 lg:py-7">
				<div className="mx-auto max-w-md px-2 text-center sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<h2 className="text-base font-semibold uppercase tracking-wider text-indigo-600">
						{header}
					</h2>

					<Heading className="mt-1" level={2}>
						{description}
					</Heading>

					<div className="mt-5">
						<div className={cx('grid grid-cols-1 gap-4', columnClasses[columns])}>
							{features.map((feature, index) => {
								const isFutureRelease =
									feature.status === 'coming-soon' || feature.status === 'in-development';
								const iconIndex = index + 1;
								let iconColor = 'text-blue-500';

								if (iconIndex % 4 === 0) {
									iconColor = 'text-purple-500';
								}

								if (iconIndex % 3 === 0) {
									iconColor = 'text-violet-500';
								}

								if (iconIndex % 2 === 0) {
									iconColor = 'text-indigo-500';
								}

								return (
									<div key={feature.title} className={cx('pt-6', isFutureRelease && 'opacity-80')}>
										<div className="flow-root rounded-lg bg-gray-50 px-3 pb-3">
											<div className="-mt-3">
												<div>
													<Icon
														icon={feature.icon}
														className={cx(
															'inline-flex items-center justify-center text-5xl',
															iconColor,
														)}
													/>
												</div>

												<Heading className="mt-2" level={4}>
													{feature.title}
												</Heading>

												{feature.status && (
													<p>
														<FeatureStatus status={feature.status} />
													</p>
												)}

												<Text className="mt-2" variant="muted">
													{feature.description}
												</Text>
											</div>
										</div>
									</div>
								);
							})}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
