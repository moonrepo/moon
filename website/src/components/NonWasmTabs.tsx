import kebabCase from 'lodash/kebabCase';
import TOML from 'smol-toml';
import YAML from 'yaml';
import CodeBlock from '@theme/CodeBlock';
import TabItem from '@theme/TabItem';
import Tabs from '@theme/Tabs';

function convertToKebabCase(data: object, parentKey?: string): object {
	const result: Record<string, unknown> = {};

	Object.entries(data).forEach(([key, value]) => {
		const childKey = parentKey === 'arch' || parentKey === 'exes' ? key : kebabCase(key);

		result[childKey] =
			value && typeof value === 'object' && !Array.isArray(value)
				? convertToKebabCase(value as object, key)
				: value;
	});

	return result;
}

export interface NonWasmTabsProps {
	data: object;
	title?: string;
}

export default function NonWasmTabs({ data = {}, title }: NonWasmTabsProps) {
	return (
		<Tabs
			groupId="non-wasm-type"
			defaultValue="toml"
			values={[
				{ label: 'JSON', value: 'json' },
				{ label: 'TOML', value: 'toml' },
				{ label: 'YAML', value: 'yaml' },
			]}
		>
			<TabItem value="json">
				<CodeBlock language="json" title={`${title}.json`}>
					{JSON.stringify(data, null, 2)}
				</CodeBlock>
			</TabItem>
			<TabItem value="toml">
				<CodeBlock language="toml" title={`${title}.toml`}>
					{TOML.stringify(convertToKebabCase(data))}
				</CodeBlock>
			</TabItem>
			<TabItem value="yaml">
				<CodeBlock language="yaml" title={`${title}.yaml`}>
					{YAML.stringify(data, { defaultKeyType: 'PLAIN', defaultStringType: 'QUOTE_SINGLE' })}
				</CodeBlock>
			</TabItem>
		</Tabs>
	);
}
