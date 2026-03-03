import type { ActionNode } from '@moonrepo/types';
import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';
import klay from 'cytoscape-klay';
import type { GraphInfo } from './types';

cytoscape.use(dagre);
cytoscape.use(klay);

function getActionLabel(node: ActionNode) {
	switch (node.action) {
		case 'sync-workspace':
			return 'SyncWorkspace';

		case 'sync-project':
			return `SyncProject(${node.params.projectId})`;

		case 'setup-proto':
			return `SetupProto(${node.params.version})`;

		case 'setup-environment':
			return `SetupEnvironment(${node.params.toolchainId}${node.params.root ? `, ${node.params.root}` : ''})`;

		case 'setup-toolchain':
			return `SetupToolchain(${node.params.toolchain.id}${node.params.toolchain.req ? `:${node.params.toolchain.req}` : ''})`;

		case 'install-dependencies':
			return `InstallDependencies(${node.params.toolchainId}${node.params.root ? `, ${node.params.root}` : ''})`;

		case 'run-task':
			if (node.params.persistent) {
				return `RunPersistentTask(${node.params.target})`;
			} else if (node.params.interactive) {
				return `RunInteractiveTask(${node.params.target})`;
			} else {
				return `RunTask(${node.params.target})`;
			}
	}
}

function getActionType(label: string) {
	if (label === 'SyncWorkspace') {
		return 'sync-workspace';
	}

	if (label.startsWith('Run') && (label.includes('Target') || label.includes('Task'))) {
		return 'run-task';
	}

	if (label.startsWith('Sync') && label.includes('Project')) {
		return 'sync-project';
	}

	if (label.startsWith('Install') && (label.includes('Deps') || label.includes('Dependencies'))) {
		return 'install-dependencies';
	}

	if (label.startsWith('Setup') && label.includes('Env')) {
		return 'setup-environment';
	}

	if (label.startsWith('Setup') && label.includes('Tool')) {
		return 'setup-toolchain';
	}

	return 'unknown';
}

function getShortDepLabel(label: string) {
	if (label === 'production') {
		return 'prod';
	}

	if (label === 'development') {
		return 'dev';
	}

	return label;
}

function extractNodes(data: GraphInfo) {
	// v2
	if ('graph' in data) {
		return data.graph.nodes.map((node, index) => {
			let row = { id: String(index), label: '', type: 'unknown' };

			if ('action' in node) {
				row.label = getActionLabel(node);
				row.type = node.action;
			} else if ('target' in node) {
				row.label = node.target;
			} else if ('id' in node) {
				row.label = node.id;
			}

			return { data: row };
		});
	}

	// v1
	return data.nodes.map((n) => ({
		data: { id: n.id.toString(), label: n.label, type: getActionType(n.label) },
	}));
}

function extractEdges(data: GraphInfo) {
	// v2
	if ('graph' in data) {
		return data.graph.edges.map((edge) => ({
			data: {
				id: `${edge[0]} -> ${edge[1]}`,
				label: getShortDepLabel(edge[2]),
				source: String(edge[0]),
				target: String(edge[1]),
			},
		}));
	}

	return data.edges.map((e) => ({
		data: {
			id: e.id,
			label: getShortDepLabel(e.label),
			source: String(e.source),
			target: String(e.target),
		},
	}));
}

export function render(element: HTMLElement, data: GraphInfo, layout: string) {
	const nodes = extractNodes(data);
	const edges = extractEdges(data);

	// https://js.cytoscape.org/
	return cytoscape({
		container: element,
		elements: { edges, nodes },
		layout: {
			fit: true,
			name: layout as 'cose',
			nodeDimensionsIncludeLabels: true,
			spacingFactor: 1,
		},
		style: [
			{
				selector: 'edges',
				style: {
					'arrow-scale': 2,
					color: '#e4f7fb',
					'curve-style': 'straight',
					'font-size': 12,
					label: 'data(label)',
					'line-cap': 'round',
					'line-color': '#c9eef6', // '#012a4a',
					'line-opacity': 0.18,
					'overlay-color': '#c9eef6',
					'target-arrow-color': '#c9eef6', // '#1a3f5c',
					'target-arrow-shape': 'chevron',
					'text-opacity': 0.6,
					width: 3,
				},
			},
			{
				selector: 'node',
				style: {
					'background-fill': 'linear-gradient',
					'background-gradient-direction': 'to-bottom-right',
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#d7dfe9 #bdc9db #97a1af',
					color: '#fff',
					height: 65,
					label: 'data(label)',
					'overlay-color': '#99aab7',
					'overlay-shape': 'ellipse',
					padding: '0',
					shape: 'ellipse',
					'text-halign': 'center',
					'text-margin-y': 6,
					'text-valign': 'bottom',
					'underlay-shape': 'ellipse',
					width: 65,
				},
			},
			{
				selector: 'node[type="run-task"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#6e58d1 #4a2ec6 #3b259e',
				},
			},
			{
				selector: 'node[type="sync-project"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ffafff #ff79ff #cc61cc',
					height: 80,
					width: 80,
				},
			},
			{
				selector: 'node[type="install-dependencies"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#afe6f2 #79d5e9 #61aaba',
					height: 80,
					width: 80,
				},
			},
			{
				selector: 'node[type="setup-environment"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#c9e166 #b7d733 #a5cd00',
					height: 90,
					width: 90,
				},
			},
			{
				selector: 'node[type="setup-toolchain"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ff9da6 #ff5b6b #cc4956',
					height: 100,
					width: 100,
				},
			},
			{
				selector: 'node[type="setup-proto"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ffafff #ff79ff #cc61cc',
					height: 110,
					width: 110,
				},
			},
			{
				selector: 'node[type="sync-workspace"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#b7a9f9 #9a87f7 #8c75f5',
					height: 120,
					width: 120,
				},
			},
		],
	});
}
