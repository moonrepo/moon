import React from 'react';
import cx from 'clsx';
import { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import Icon from '@site/src/ui/iconography/Icon';
import Heading from '@site/src/ui/typography/Heading';
import Text from '@site/src/ui/typography/Text';

export interface FeatureItem {
	title: string;
	description: string;
	icon: IconDefinition;
}

export interface FeatureSectionProps {
	children: React.ReactNode;
	color: string;
	description: React.ReactNode;
	items: FeatureItem[];
	reversed?: boolean;
	title: string;
	suptitle: string;
}

export default function FeatureSection({
	children,
	color,
	description,
	items,
	reversed,
	title,
	suptitle,
}: FeatureSectionProps) {
	return (
		<div className="relative py-4 sm:py-5 lg:py-6">
			<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
				<div className="bg-white rounded-lg p-6">
					<div
						className={cx(
							'grid grid-cols-1 md:grid-cols-2 gap-8 items-center',
							reversed && 'flex-row-reverse',
						)}
					>
						<div>
							<Heading level={5} className={color}>
								{suptitle}
							</Heading>

							<Heading level={2} className="mt-1">
								{title}
							</Heading>

							<p className={children ? 'my-4' : 'mt-4'}>
								<Text size="lg">{description}</Text>
							</p>

							{children}
						</div>

						<aside>
							<ul className="flex flex-col gap-4 m-0 p-0">
								{items.map((item) => (
									<li className="relative list-none pl-5">
										<Heading level={5} className="mb-1">
											{item.title}
										</Heading>

										<p className="m-0">{item.description}</p>

										<div className="absolute top-1 left-0">
											<Icon
												icon={item.icon}
												className={cx('text-2xl justify-center flex', color)}
												style={{ maxWidth: 54 }}
											/>
										</div>
									</li>
								))}
							</ul>
						</aside>
					</div>
				</div>
			</div>
		</div>
	);
}
