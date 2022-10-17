import React from 'react';
import cx from 'clsx';
import { faChevronLeft, faChevronRight } from '@fortawesome/pro-regular-svg-icons';
import type { Props } from '@theme/PaginatorNavLink';
import Icon from '../ui/iconography/Icon';
import Link from '../ui/typography/Link';

export default function PaginatorNavLink({ permalink, title, isNext }: Props) {
	return (
		<div className={cx('flex-1', isNext ? 'text-right' : 'text-left')}>
			<Link weight="bold" to={permalink}>
				{!isNext && <Icon className="mr-1 icon-previous" icon={faChevronLeft} />}
				{title}
				{isNext && <Icon className="ml-1 icon-next" icon={faChevronRight} />}
			</Link>
		</div>
	);
}
