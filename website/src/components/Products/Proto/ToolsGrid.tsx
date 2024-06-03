import { useEffect, useState } from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import Text from '@site/src/ui/typography/Text';
import Bun from '@site/static/img/tools/bun.svg';
import Deno from '@site/static/img/tools/deno.svg';
import Go from '@site/static/img/tools/go.svg';
import Node from '@site/static/img/tools/node.svg';
import Python from '@site/static/img/tools/python.svg';
import Rust from '@site/static/img/tools/rust.svg';
import { loadToolsData, ProtoTool } from '../../../data/proto-tools';

export interface ToolsGridProps {
	cols?: number;
}

export default function ToolsGrid({ cols = 3 }: ToolsGridProps) {
	const [tools, setTools] = useState<ProtoTool[]>([]);

	useEffect(() => {
		// eslint-disable-next-line promise/prefer-await-to-then, no-console
		loadToolsData('third-party').then(setTools).catch(console.error);
	}, []);

	return (
		<div>
			<div className={clsx('grid gap-4 px-4', cols === 6 ? 'grid-cols-6' : 'grid-cols-3')}>
				<div className="text-center">
					<Link href="/docs/proto/tools#bun">
						<Bun width="100%" className="inline-block" />
					</Link>
				</div>

				<div className="text-center">
					<Link href="/docs/proto/tools#deno">
						<Deno width="100%" className="inline-block" />
					</Link>
				</div>

				<div className="text-center">
					<Link href="/docs/proto/tools#go">
						<Go width="100%" className="inline-block" />
					</Link>
				</div>

				<div className="text-center">
					<Link href="/docs/proto/tools#node">
						<Node width="100%" className="inline-block" />
					</Link>

					<Text className="m-0" variant="muted">
						+ npm, pnpm, yarn
					</Text>
				</div>

				<div className="text-center">
					<Link href="/docs/proto/tools#python">
						<Python width="100%" className="inline-block" />
					</Link>
				</div>

				<div className="text-center">
					<Link href="/docs/proto/tools#rust">
						<Rust width="100%" className="inline-block" />
					</Link>
				</div>
			</div>

			<div className="text-center mt-3">
				<Text weight="bold" variant="muted">
					<Link href="/docs/proto/tools#third-party">
						...with {tools.length} more and growing...
					</Link>
				</Text>
			</div>
		</div>
	);
}
