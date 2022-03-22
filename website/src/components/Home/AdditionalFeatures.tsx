import React from 'react';
import { FeaturesProps } from './Features';
import FeatureStatus from './FeatureStatus';
import Icon from '../Icon';

export type AdditionalFeaturesProps = Omit<FeaturesProps, 'columns'>;

export default function AdditionalFeatures({
	header,
	description,
	features,
}: AdditionalFeaturesProps) {
	return (
		<div className="bg-white">
			<div className=" max-w-7xl mx-auto py-16 px-6 sm:px-8 lg:py-20 lg:px-10">
				<div className="max-w-3xl mx-auto text-center">
					<h2 className="text-3xl font-extrabold text-gray-900">{header}</h2>
					<p className="mt-4 text-lg text-gray-500">{description}</p>
				</div>
				<dl className="mt-12 space-y-10 sm:space-y-0 sm:grid sm:grid-cols-2 sm:gap-x-6 sm:gap-y-12 lg:grid-cols-4 lg:gap-x-8">
					{features.map((feature) => (
						<div key={feature.title} className="relative">
							<dt>
								<span className="absolute h-6 w-6 text-indigo-500" aria-hidden="true">
									<Icon icon={feature.icon} />
								</span>
								<p className="ml-9 text-lg leading-6 font-medium text-gray-900">{feature.title}</p>
							</dt>

							<dd className="mt-2 ml-9 text-base text-gray-600">
								{feature.status && (
									<p className="mb-2">
										<FeatureStatus status={feature.status} />
									</p>
								)}

								{feature.description}
							</dd>
						</div>
					))}
				</dl>
			</div>
		</div>
	);
}
