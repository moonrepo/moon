import React from 'react';
import { FeaturesProps } from './Features';
import FeatureStatus from './FeatureStatus';
import Icon from '../Icon';
import Heading from '../Heading';
import Text from '../Text';

export type AdditionalFeaturesProps = Omit<FeaturesProps, 'description' | 'columns'>;

export default function AdditionalFeatures({ header, features }: AdditionalFeaturesProps) {
	return (
		<div className="bg-white">
			<div className="max-w-7xl mx-auto py-16 px-6 sm:px-8 lg:py-20 lg:px-10">
				<div className="max-w-3xl mx-auto text-center">
					<Heading level={3}>{header}</Heading>
				</div>

				<dl className="mt-12 space-y-10 sm:space-y-0 sm:grid sm:grid-cols-2 sm:gap-x-6 sm:gap-y-12 lg:grid-cols-4 lg:gap-x-8">
					{features.map((feature) => (
						<div key={feature.title} className="relative">
							<dt>
								<Icon icon={feature.icon} className="absolute h-6 w-6 text-indigo-500" />

								<Heading className="ml-9" level={5}>
									{feature.title}
								</Heading>
							</dt>

							<Text as="dd" className="mt-2 ml-9" variant="muted">
								{feature.status && (
									<p className="mb-2">
										<FeatureStatus status={feature.status} />
									</p>
								)}

								{feature.description}
							</Text>
						</div>
					))}
				</dl>
			</div>
		</div>
	);
}
