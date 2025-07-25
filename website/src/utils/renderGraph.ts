import cytoscape from 'cytoscape';
import dagre from 'cytoscape-dagre';

cytoscape.use(dagre);

export function renderGraph(element: HTMLElement, graph: cytoscape.ElementsDefinition) {
	return cytoscape({
		container: element,
		elements: graph,
		layout: {
			// @ts-expect-error Types incorrect
			fit: true,
			name: 'dagre',
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
					'line-opacity': 0.25,
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
				selector: 'node[type="run-target"]',
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
					height: 90,
					width: 90,
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
				selector: 'node[id="sync-workspace"]',
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
