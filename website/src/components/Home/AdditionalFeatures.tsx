import React from 'react';
import Heading from '../../ui/typography/Heading';
import Icon from '../../ui/typography/Icon';
import Text from '../../ui/typography/Text';
import { FeaturesProps } from './Features';
import FeatureStatus from './FeatureStatus';

export type AdditionalFeaturesProps = Omit<FeaturesProps, 'columns' | 'description'>;

export default function AdditionalFeatures({ header, features }: AdditionalFeaturesProps) {
	return (
		<div className="bg-white">
			<div className="relative py-4 sm:py-5 lg:py-6">
				<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<Heading align="center" level={3}>
						{header}
					</Heading>

					<dl className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-4 lg:gap-6">
						{features.map((feature) => (
							<div key={feature.title} className="relative">
								<dt>
									<Icon icon={feature.icon} className="absolute h-3 w-3 text-indigo-500" />

									<Heading className="ml-4" level={5}>
										{feature.title}
									</Heading>
								</dt>

								<Text as="dd" className="mt-1 ml-4" variant="muted">
									{feature.status && (
										<p className="mb-1">
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
		</div>
	);
}
