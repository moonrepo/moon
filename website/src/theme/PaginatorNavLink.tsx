import React from 'react';
import cx from 'clsx';
import { faChevronLeft, faChevronRight } from '@fortawesome/pro-regular-svg-icons';
import type { Props } from '@theme/PaginatorNavLink';
import Icon from '../ui/iconography/Icon';
import Link from '../ui/typography/Link';

export default function PaginatorNavLink({ permalink, title, isNext }: Props) {
	return (
		<Link className={cx('grow', isNext && 'text-right')} weight="bold" to={permalink}>
			{!isNext && <Icon className="mr-1 icon-previous" icon={faChevronLeft} />}
			{title}
			{isNext && <Icon className="ml-1 icon-next" icon={faChevronRight} />}
		</Link>
	);
}
