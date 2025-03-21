import cx from 'clsx';
import type { Props } from '@theme/PaginatorNavLink';
import Icon from '../ui/iconography/Icon';
import Link from '../ui/typography/Link';

export default function PaginatorNavLink({ permalink, title, isNext }: Props) {
	return (
		<div className={cx('flex-1', isNext ? 'text-right' : 'text-left')}>
			<Link weight="bold" to={permalink}>
				{!isNext && <Icon className="mr-1 icon-previous" icon="material-symbols:chevron-left-rounded" />}
				{title}
				{isNext && <Icon className="ml-1 icon-next" icon="material-symbols:chevron-right-rounded" />}
			</Link>
		</div>
	);
}
