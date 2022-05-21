import React from 'react';
import CodeBlock from '@theme/CodeBlock';
import TabItem from '@theme/TabItem';
import Tabs from '@theme/Tabs';

export interface AddDepsTabsProps {
	dep: string;
	dev?: string;
	peer?: string;
}

function getYarn(props: AddDepsTabsProps, workspaces: boolean, v1: boolean) {
	let cmd = `yarn add `;

	if (props.dep) {
		cmd += '--dev ';
	} else if (props.peer) {
		cmd += '--peer ';
	}

	if (v1 && workspaces) {
		cmd += '-W ';
	}

	cmd += props.dep;

	return cmd;
}

function getNpm(props: AddDepsTabsProps) {
	let cmd = `npm install `;

	if (props.dep) {
		cmd += '--save-dev ';
	} else if (props.peer) {
		cmd += '--save-peer ';
	}

	cmd += props.dep;

	return cmd;
}

function getPnpm(props: AddDepsTabsProps, workspaces: boolean) {
	let cmd = `pnpm add `;

	if (props.dep) {
		cmd += '--save-dev ';
	} else if (props.peer) {
		cmd += '--save-peer ';
	}

	if (workspaces) {
		cmd += '-w ';
	}

	cmd += props.dep;

	return cmd;
}

export default function AddDepsTabs(props: AddDepsTabsProps) {
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
				<CodeBlock language="shell">{getYarn(props, false, false)}</CodeBlock>
			</TabItem>
			<TabItem value="yarn1">
				<CodeBlock language="shell">
					{getYarn(props, false, true)}
					{'\n\n# If using workspaces\n'}
					{getYarn(props, true, true)}
				</CodeBlock>
			</TabItem>
			<TabItem value="npm">
				<CodeBlock language="shell">{getNpm(props)}</CodeBlock>
			</TabItem>
			<TabItem value="pnpm">
				<CodeBlock language="shell">
					{getPnpm(props, false)}
					{'\n\n# If using workspaces\n'}
					{getPnpm(props, true)}
				</CodeBlock>
			</TabItem>
		</Tabs>
	);
}
