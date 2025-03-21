import cx from 'clsx';
import type { IconifyIcon } from '@iconify/react';
import Icon from '@site/src/ui/iconography/Icon';
import Heading from '@site/src/ui/typography/Heading';
import Text from '@site/src/ui/typography/Text';
import CTA, { CTAProps } from './CTA';

export interface ProductItem {
	title: string;
	description: React.ReactNode;
	icon: IconifyIcon | string;
}

export interface ProductSectionProps {
	children?: React.ReactNode;
	color: string;
	cta?: CTAProps;
	description: React.ReactNode;
	id: string;
	items?: ProductItem[];
	logo: React.ReactNode;
	reversed?: boolean;
	stretched?: boolean;
	suptitle: string;
	title: string;
}

export default function ProductSection({
	children,
	color,
	cta,
	description,
	id,
	items,
	logo,
	reversed,
	stretched,
	suptitle,
	title,
}: ProductSectionProps) {
	return (
		<div className="relative py-4 sm:py-5 lg:py-6" id={id}>
			<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
				<div className="bg-white rounded-lg p-6 drop-shadow">
					<div
						className={cx(
							'grid grid-cols-1 md:grid-cols-2 gap-8',
							stretched ? 'items-stretch' : 'items-center',
						)}
					>
						<div className={reversed ? 'order-2' : 'order-1'}>
							<Heading level={5} className={color}>
								{suptitle}
							</Heading>

							<div className="flex items-center sm:items-end gap-2 mt-1">
								<div>{logo}</div>

								<div className="self-center">
									<Heading level={2} className="mt-1 text-slate-900">
										{title}
									</Heading>
								</div>
							</div>

							<p className={children ? 'my-4' : 'mt-3'}>
								<Text as="span" size="lg" className="text-slate-900">
									{description}
								</Text>
							</p>

							{cta && <CTA {...cta} />}
						</div>

						<aside className={reversed ? 'order-1' : 'order-2'}>
							{items && (
								<ul className="flex flex-col gap-4 m-0 p-0">
									{items.map((item) => (
										<li key={item.title} className="relative list-none pl-5">
											<Heading level={5} className="mb-1 text-slate-900">
												{item.title}
											</Heading>

											<p className="m-0 text-slate-900">{item.description}</p>

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
							)}

							{children}
						</aside>
					</div>
				</div>
			</div>
		</div>
	);
}
