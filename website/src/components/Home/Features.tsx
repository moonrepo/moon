import React from 'react';
import cx from 'clsx';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Icon from '../Icon';
import FeatureStatus, { StatusType } from './FeatureStatus';

export interface FeaturesProps {
	header: string;
	description: string;
	features: {
		title: string;
		icon: IconDefinition;
		description: string;
		status?: StatusType;
	}[];
	columns?: 3 | 4 | 5;
}

const columnClasses = {
	3: 'sm:grid-cols-2 lg:grid-cols-3',
	4: 'sm:grid-cols-2 lg:grid-cols-4',
	5: 'sm:grid-cols-3 lg:grid-cols-5',
};

export default function Features({ header, description, features, columns = 4 }: FeaturesProps) {
	return (
		<div className="relative bg-white py-16 sm:py-12 lg:py-20">
			<div className="mx-auto max-w-md px-4 text-center sm:max-w-3xl sm:px-6 lg:max-w-7xl lg:px-8">
				<h2 className="text-base font-semibold uppercase tracking-wider text-indigo-600">
					{header}
				</h2>
				<p className="mt-2 text-3xl font-extrabold tracking-tight text-gray-900 sm:text-4xl">
					{description}
				</p>
				<div className="mt-12">
					<div className={`grid grid-cols-1 gap-8 ${columnClasses[columns]}`}>
						{features.map((feature) => {
							const isFutureRelease =
								feature.status === 'coming-soon' || feature.status === 'in-development';

							return (
								<div key={feature.title} className={cx('pt-6', isFutureRelease && 'opacity-80')}>
									<div className="flow-root rounded-lg bg-gray-50 px-6 pb-8">
										<div className="-mt-6">
											<div>
												<span className="inline-flex items-center justify-center text-5xl text-indigo-500">
													<Icon icon={feature.icon} />
												</span>
											</div>
											<h3 className="mt-4 text-xl font-semibold tracking-tight text-gray-900">
												{feature.title}
											</h3>
											{feature.status && (
												<p>
													<FeatureStatus status={feature.status} />
												</p>
											)}
											<p className="mt-4 text-base text-gray-600">{feature.description}</p>
										</div>
									</div>
								</div>
							);
						})}
					</div>
				</div>
			</div>
		</div>
	);
}
