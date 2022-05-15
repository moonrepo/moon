import React from 'react';
import { FooterLinkItem } from '@docusaurus/theme-common';
import useBaseUrl from '@docusaurus/useBaseUrl';
import type { Props } from '@theme/Footer/Links/MultiColumn';
import Heading from '../../../ui/typography/Heading';
import Link from '../../../ui/typography/Link';

function ColumnLinkItem({ item }: { item: FooterLinkItem }) {
	const { to, href, label, prependBaseUrlToHref, ...props } = item;
	const toUrl = useBaseUrl(to);
	const normalizedHref = useBaseUrl(href, {
		forcePrependBaseUrl: true,
	});

	return (
		<Link
			{...(href
				? {
						href: prependBaseUrlToHref ? normalizedHref : href,
				  }
				: {
						to: toUrl,
				  })}
			{...props}
		>
			{label}
		</Link>
	);
}

function Column({ column }: { column: Props['columns'][number] }) {
	return (
		<div>
			<Heading level={6} transform="uppercase">
				{column.title}
			</Heading>

			<ul role="list" className="m-0 mt-2 p-0 space-y-1 list-none">
				{column.items.map((item) => (
					<li key={item.href ?? item.to}>
						<ColumnLinkItem item={item} />
					</li>
				))}
			</ul>
		</div>
	);
}

export default function FooterLinksMultiColumn({ columns }: Props) {
	return (
		<>
			{columns.map((column, i) => (
				<Column key={i} column={column} />
			))}
		</>
	);
}
