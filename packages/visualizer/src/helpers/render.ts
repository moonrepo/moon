import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';
import klay from 'cytoscape-klay';
import type { GraphInfo } from './types';

cytoscape.use(dagre);
cytoscape.use(klay);

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

export function render(element: HTMLElement, data: GraphInfo, layout: string) {
	const nodes = data.nodes.map((n) => ({
		data: { id: n.id.toString(), label: n.label, type: getActionType(n.label) },
	}));

	const edges = data.edges.map((e) => ({
		data: {
			id: e.id.toString(),
			label: getShortDepLabel(e.label),
			source: e.source.toString(),
			target: e.target.toString(),
		},
	}));

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
					height: 60,
					label: 'data(label)',
					'overlay-color': '#99aab7',
					'overlay-shape': 'ellipse',
					padding: '0',
					shape: 'ellipse',
					'text-halign': 'center',
					'text-margin-y': 6,
					'text-valign': 'bottom',
					'underlay-shape': 'ellipse',
					width: 60,
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
					height: 100,
					width: 100,
				},
			},
			{
				selector: 'node[type="setup-environment"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#c9e166 #b7d733 #a5cd00',
					height: 100,
					width: 100,
				},
			},
			{
				selector: 'node[type="setup-toolchain"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ff9da6 #ff5b6b #cc4956',
					height: 120,
					width: 120,
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
