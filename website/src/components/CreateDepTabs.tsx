import React from 'react';
import CodeBlock from '@theme/CodeBlock';
import TabItem from '@theme/TabItem';
import Tabs from '@theme/Tabs';

export interface CreateDepTabsProps {
	args?: string[];
	dep: string; // Dep to create
}

function createDep(manager: string, dep: string, args: string[]) {
	return `${manager} create ${dep} ${args.join(' ')}`.trim();
}

export default function CreateDepTabs({ dep, args = [] }: CreateDepTabsProps) {
	return (
		<Tabs
			groupId="package-manager"
			defaultValue="yarn"
			values={[
				{ label: 'Yarn', value: 'yarn' },
				{ label: 'Yarn (classic)', value: 'yarn1' },
				{ label: 'npm', value: 'npm' },
				{ label: 'pnpm', value: 'pnpm' },
			]}
		>
			<TabItem value="yarn">
				<CodeBlock language="shell">{createDep('yarn', dep, args)}</CodeBlock>
			</TabItem>
			<TabItem value="yarn1">
				<CodeBlock language="shell">{createDep('yarn', dep, args)}</CodeBlock>
			</TabItem>
			<TabItem value="npm">
				<CodeBlock language="shell">{createDep('npm', dep, args)}</CodeBlock>
			</TabItem>
			<TabItem value="pnpm">
				<CodeBlock language="shell">{createDep('pnpm', dep, args)}</CodeBlock>
			</TabItem>
		</Tabs>
	);
}
