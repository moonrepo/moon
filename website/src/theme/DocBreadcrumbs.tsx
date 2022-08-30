/* eslint-disable @typescript-eslint/no-unsafe-call */

import React from 'react';
// @ts-expect-error Not typed!
import { useHomePageRoute, useSidebarBreadcrumbs } from '@docusaurus/theme-common/internal';
import useBaseUrl from '@docusaurus/useBaseUrl';
import { faAngleRight, faHouseBlank } from '@fortawesome/pro-regular-svg-icons';
import Icon from '../ui/iconography/Icon';
import Link from '../ui/typography/Link';
import Text from '../ui/typography/Text';

interface BreadcrumbsItemLinkProps {
	active: boolean;
	children: string;
	href?: string;
}

function BreadcrumbsItemLink({ active, children, href }: BreadcrumbsItemLinkProps) {
	return href ? (
		<Link
			aria-current={active ? 'page' : undefined}
			href={href}
			itemProp="item"
			size="sm"
			variant="muted"
			weight="medium"
		>
			<span itemProp="name">{children}</span>
		</Link>
	) : (
		<Text
			aria-current={active ? 'page' : undefined}
			as="span"
			itemProp="item name"
			size="sm"
			variant="muted"
			weight="medium"
		>
			{children}
		</Text>
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
				<Icon icon={faAngleRight} className="flex-shrink-0 text-gray-600 mr-2" aria-hidden="true" />

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
			<Link href={homeHref} variant="muted">
				<Icon icon={faHouseBlank} className="flex-shrink-0" aria-hidden="true" />
				<span className="sr-only">Home</span>
			</Link>
		</li>
	);
}

export default function DocBreadcrumbs() {
	const breadcrumbs = useSidebarBreadcrumbs() as { href: string; label: string }[];
	const homePageRoute = useHomePageRoute() as object | undefined;

	if (!breadcrumbs) {
		return null;
	}

	return (
		<nav className="flex pl-1 mb-2" aria-label="Breadcrumb">
			<ol
				role="list"
				className="list-none p-0 m-0 flex items-center space-x-2"
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
