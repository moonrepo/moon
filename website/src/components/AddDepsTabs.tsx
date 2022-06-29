import React from 'react';
import CodeBlock from '@theme/CodeBlock';
import TabItem from '@theme/TabItem';
import Tabs from '@theme/Tabs';

export interface AddDepsTabsProps {
	dep: string; // Dep to add
	dev?: string;
	package?: string; // Workspace package to add to
	peer?: string;
}

function getYarn(props: AddDepsTabsProps, workspaces: boolean, v1: boolean) {
	let cmd = props.package ? `yarn workspace ${props.package} add ` : `yarn add `;

	if (props.dev) {
		cmd += '--dev ';
	} else if (props.peer) {
		cmd += '--peer ';
	}

	if (v1 && workspaces && !props.package) {
		cmd += '-W ';
	}

	cmd += props.dep;

	return cmd;
}

function getNpm(props: AddDepsTabsProps) {
	let cmd = `npm install `;

	if (props.dev) {
		cmd += '--save-dev ';
	} else if (props.peer) {
		cmd += '--save-peer ';
	}

	if (props.package) {
		cmd += `--workspace ${props.package} `;
	}

	cmd += props.dep;

	return cmd;
}

function getPnpm(props: AddDepsTabsProps, workspaces: boolean) {
	let cmd = `pnpm add `;

	if (props.dev) {
		cmd += '--save-dev ';
	} else if (props.peer) {
		cmd += '--save-peer ';
	}

	if (props.package) {
		cmd += `--filter ${props.package} `;
	} else if (workspaces) {
		cmd += '-w ';
	}

	cmd += props.dep;

	return cmd;
}

export default function AddDepsTabs(props: AddDepsTabsProps) {
	let yarn1 = getYarn(props, false, true);
	let pnpm = getPnpm(props, false);

	if (!props.package) {
		yarn1 += '\n\n# If using workspaces\n';
		pnpm += '\n\n# If using workspaces\n';

		yarn1 += getYarn(props, true, true);
		pnpm += getPnpm(props, true);
	}

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
				<CodeBlock language="shell">{yarn1}</CodeBlock>
			</TabItem>
			<TabItem value="npm">
				<CodeBlock language="shell">{getNpm(props)}</CodeBlock>
			</TabItem>
			<TabItem value="pnpm">
				<CodeBlock language="shell">{pnpm}</CodeBlock>
			</TabItem>
		</Tabs>
	);
}
