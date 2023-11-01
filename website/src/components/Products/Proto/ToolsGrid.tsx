import clsx from 'clsx';
import Link from '@docusaurus/Link';
import Text from '@site/src/ui/typography/Text';

export interface ToolsGridProps {
	cols?: number;
}

export default function ToolsGrid({ cols = 3 }: ToolsGridProps) {
	return (
		<div className={clsx('grid gap-4 px-4', cols === 6 ? 'grid-cols-6' : 'grid-cols-3')}>
			<div className="text-center">
				<Link href="/docs/proto/tools#bun">
					<img src="/img/tools/bun.svg" width="100%" className="inline-block" />
				</Link>
			</div>

			<div className="text-center">
				<Link href="/docs/proto/tools#deno">
					<img src="/img/tools/deno.svg" width="100%" className="inline-block" />
				</Link>
			</div>

			<div className="text-center">
				<Link href="/docs/proto/tools#go">
					<img src="/img/tools/go.svg" width="100%" className="inline-block" />
				</Link>
			</div>

			<div className="text-center">
				<Link href="/docs/proto/tools#node">
					<img src="/img/tools/node.svg" width="100%" className="inline-block" />
				</Link>

				<Text className="m-0" variant="muted">
					+ npm, pnpm, yarn
				</Text>
			</div>

			<div className="text-center">
				<Link href="/docs/proto/tools#python">
					<img src="/img/tools/python.svg" width="100%" className="inline-block" />
				</Link>
			</div>

			<div className="text-center">
				<Link href="/docs/proto/tools#rust">
					<img src="/img/tools/rust.svg" width="100%" className="inline-block" />
				</Link>
			</div>
		</div>
	);
}
