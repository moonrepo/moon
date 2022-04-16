import React from 'react';
import { faChevronLeft, faChevronRight } from '@fortawesome/pro-regular-svg-icons';
import type { Props } from '@theme/PaginatorNavLink';
import Icon from '../ui/iconography/Icon';
import Link from '../ui/typography/Link';

export default function PaginatorNavLink({ permalink, title }: Props) {
	return (
		<Link className="grow" weight="bold" to={permalink}>
			<Icon className="mr-1 icon-previous" icon={faChevronLeft} />
			{title}
			<Icon className="ml-1 icon-next" icon={faChevronRight} />
		</Link>
	);
}
