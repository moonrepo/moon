import Link from '@docusaurus/Link';
import Heading from '@site/src/ui/typography/Heading';
import Text from '@site/src/ui/typography/Text';
import CodeBlock from '@theme/CodeBlock';
import Code from '@theme/MDXComponents/Code';
import { ProtoTool } from '../../../data/proto-tools';
import TomlLink from '../../Docs/TomlLink';
import WasmLink from '../../Docs/WasmLink';

export interface ToolCardProps {
	id: string;
	tool: ProtoTool;
}

// eslint-disable-next-line complexity
export default function ToolCard({ id, tool }: ToolCardProps) {
	const bins = tool.bins ?? [];
	const dirs = tool.globalsDirs ?? [];
	const detect = tool.detectionSources ?? [];
	const usageId = tool.usageId ?? id;
	let usage = `proto install ${usageId}`;

	if (tool.pluginLocator) {
		usage = `proto add-plugin ${usageId} "${tool.pluginLocator}"\n${usage}`;
	}

	return (
		<div className="relative rounded-lg px-2 py-2 text-black border-solid border border-t-0 border-b-2 bg-gray-50 border-gray-200/75 dark:bg-slate-700 dark:border-slate-900/75">
			{tool.pluginType === 'toml' && <TomlLink to={tool.repoUrl} noMargin />}
			{tool.pluginType === 'wasm' && <WasmLink to={tool.repoUrl} noMargin />}

			<Heading level={5} className="mb-1">
				<Link href={tool.homepageUrl ?? tool.repoUrl}>{tool.name}</Link>
			</Heading>

			<Text>{tool.description}</Text>

			<CodeBlock language="shell">{usage}</CodeBlock>

			{bins.length > 0 && (
				<Text size="sm" variant="muted" className="m-0 mt-1">
					Available bins:{' '}
					{bins.map((bin, i) => (
						<>
							<Code>{bin}</Code>
							{i === bins.length - 1 ? '' : ', '}
						</>
					))}
				</Text>
			)}

			{dirs.length > 0 && (
				<Text size="sm" variant="muted" className="m-0 mt-1">
					Globals directory:{' '}
					{dirs.map((dir, i) => (
						<>
							<Code>{dir}</Code>
							{i === dirs.length - 1 ? '' : ', '}
						</>
					))}
				</Text>
			)}

			{detect.length > 0 && (
				<Text size="sm" variant="muted" className="m-0 mt-1">
					Detection sources:{' '}
					{detect.map((src, i) => {
						let content = (
							<>
								<Code>{src.file}</Code>
								{src.label ? ' ' : ''}
								{src.label}
							</>
						);

						content = src.url ? <Link href={src.url}>{content}</Link> : <span>{content}</span>;

						return (
							<>
								{content}
								{i === detect.length - 1 ? '' : ', '}
							</>
						);
					})}
				</Text>
			)}
		</div>
	);
}
