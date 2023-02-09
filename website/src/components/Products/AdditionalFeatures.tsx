import React from 'react';
import Icon from '../../ui/iconography/Icon';
import Heading from '../../ui/typography/Heading';
import Text from '../../ui/typography/Text';
import FeatureStatus from '../FeatureStatus';
import { FeaturesProps } from './Features';

export type AdditionalFeaturesProps = Omit<FeaturesProps, 'columns' | 'description' | 'tier'>;

export default function AdditionalFeatures({ header, features }: AdditionalFeaturesProps) {
	return (
		<div className="bg-white">
			<div className="relative py-4 sm:py-6 lg:py-8">
				<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<Heading align="center" className="text-gray-900" level={3}>
						{header}
					</Heading>

					<dl className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-4 lg:gap-6">
						{features.map((feature) => (
							<div key={feature.title} className="relative">
								<dt>
									<Icon
										icon={feature.icon}
										className="absolute h-3 w-3 text-purple-500"
										style={{ maxWidth: 16 }}
									/>

									<Heading className="ml-4 text-gray-900" level={5}>
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
