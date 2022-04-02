import React from 'react';
import cx from 'clsx';
import Link from '@docusaurus/Link';
import { useHomePageRoute, useSidebarBreadcrumbs } from '@docusaurus/theme-common';
import useBaseUrl from '@docusaurus/useBaseUrl';
import { faAngleRight, faHouseBlank } from '@fortawesome/pro-regular-svg-icons';
import Icon from '@site/src/ui/typography/Icon';

interface BreadcrumbsItemLinkProps {
	active: boolean;
	children: string;
	href?: string;
}

function BreadcrumbsItemLink({ active, children, href }: BreadcrumbsItemLinkProps) {
	const className = 'ml-2 text-sm font-medium text-gray-500';

	return href ? (
		<Link
			className={cx(className, 'hover:text-gray-400')}
			href={href}
			itemProp="item"
			aria-current={active ? 'page' : undefined}
		>
			<span itemProp="name">{children}</span>
		</Link>
	) : (
		<span className={className} itemProp="item name" aria-current={active ? 'page' : undefined}>
			{children}
		</span>
	);
}

interface BreadcrumbsItemProps {
	children: React.ReactNode;
	index: number;
}

function BreadcrumbsItem({ children, index }: BreadcrumbsItemProps) {
	return (
		<li itemScope itemProp="itemListElement" itemType="https://schema.org/ListItem">
			<div className="flex items-center">
				<Icon icon={faAngleRight} className="flex-shrink-0 text-gray-600" aria-hidden="true" />

				{children}

				<meta itemProp="position" content={String(index + 1)} />
			</div>
		</li>
	);
}

function HomeBreadcrumbItem() {
	const homeHref = useBaseUrl('/');

	return (
		<li>
			<div>
				<a href={homeHref} className="text-gray-500 hover:text-gray-400">
					<Icon icon={faHouseBlank} className="flex-shrink-0" aria-hidden="true" />
					<span className="sr-only">Home</span>
				</a>
			</div>
		</li>
	);
}

export default function DocBreadcrumbs() {
	const breadcrumbs = useSidebarBreadcrumbs();
	const homePageRoute = useHomePageRoute();

	if (!breadcrumbs) {
		return null;
	}

	return (
		<nav className="flex" aria-label="Breadcrumb">
			<ol
				role="list"
				className="list-none p-0 pl-1 m-0 mb-2 flex items-center space-x-2"
				itemScope
				itemType="https://schema.org/BreadcrumbList"
			>
				{homePageRoute && <HomeBreadcrumbItem />}

				{breadcrumbs.map((item, i) => (
					<BreadcrumbsItem key={i} index={i}>
						<BreadcrumbsItemLink
							href={i < breadcrumbs.length ? item.href : undefined}
							active={i === breadcrumbs.length - 1}
						>
							{item.label}
						</BreadcrumbsItemLink>
					</BreadcrumbsItem>
				))}
			</ol>
		</nav>
	);
}
