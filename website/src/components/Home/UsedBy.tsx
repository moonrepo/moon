import React from 'react';
import { faCirclePlus } from '@fortawesome/pro-duotone-svg-icons';
import DepotSvg from '../../../static/brands/depot.svg';
import Icon from '../../ui/iconography/Icon';
import Heading from '../../ui/typography/Heading';
import Link from '../../ui/typography/Link';

function onClick(event: React.MouseEvent) {
	event.stopPropagation();
	event.preventDefault();

	const select = document.querySelector<HTMLSelectElement>('#subject');
	const button = document.querySelector<HTMLButtonElement>('#contact-next');

	if (select && button) {
		select.value = 'Affiliation';
		select.dispatchEvent(new Event('change', { bubbles: true }));

		// Wait for button to become enabled
		setTimeout(() => {
			button.click();
		}, 0);
	}
}

export default function UsedBy() {
	return (
		<div className="bg-white">
			<div className="relative py-4 sm:py-6 lg:py-8">
				<div className="mx-auto max-w-md px-2 sm:max-w-3xl sm:px-3 lg:max-w-7xl lg:px-4">
					<Heading align="center" className="text-gray-900" level={3}>
						Used by these innovative companies
					</Heading>

					<div className="mt-4 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-6">
						<div className="col-span-1 flex justify-center">
							<Link to="https://depot.dev/?ref=moonrepo" variant="muted" title="Depot">
								<DepotSvg className="w-full max-w-full" />
							</Link>
						</div>

						<div className="col-span-1 flex justify-start items-center">
							<Link href="#" onClick={onClick} variant="muted" title="List your company here">
								<Icon icon={faCirclePlus} className="text-3xl" />
							</Link>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
