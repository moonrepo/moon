import React from 'react';
import DepotSvg from '../../../static/brands/depot.svg';
import Heading from '../../ui/typography/Heading';
import Link from '../../ui/typography/Link';

export default function UsedBy() {
	return (
		<div className="bg-white">
			<div className="relative py-4 sm:py-6 lg:py-8">
				<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<Heading align="center" className="text-gray-900" level={3}>
						Used by these innovative companies
					</Heading>

					<div className="mt-4 grid grid-cols-2 gap-8 md:grid-cols-5">
						<div className="col-span-1 flex justify-center md:col-span-2 lg:col-span-1">
							<Link to="https://depot.dev/?ref=moonrepo" variant="muted" title="Depot">
								<DepotSvg />
							</Link>
						</div>

						<div className="col-span-2 flex justify-center md:col-span-3 lg:col-span-1">
							<img
								className="h-12"
								src="https://tailwindui.com/img/logos/workcation-logo-gray-400.svg"
								alt="Workcation"
							/>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
