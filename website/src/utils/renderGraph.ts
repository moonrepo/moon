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
					'curve-style': 'straight',
					'line-cap': 'round',
					'line-color': '#c9eef6', // '#012a4a',
					'line-opacity': 0.25,
					'overlay-color': '#c9eef6',
					'target-arrow-color': '#c9eef6', // '#1a3f5c',
					'target-arrow-shape': 'tee',
					width: 3,
				},
			},
			{
				selector: 'node',
				style: {
					// @ts-expect-error Types incorrect
					'background-fill': 'linear-gradient',
					'background-gradient-direction': 'to-bottom-right',
					'background-gradient-stop-colors': '#d7dfe9 #bdc9db #97a1af',
					color: '#fff',
					height: 60,
					label: 'data(label)',
					'overlay-color': '#99aab7',
					'overlay-shape': 'ellipse',
					padding: 0,
					shape: 'ellipse',
					'text-halign': 'center',
					'text-margin-y': 6,
					'text-valign': 'bottom',
					'underlay-shape': 'ellipse',
					width: 60,
				},
			},
			{
				selector: 'node[type="run-task"], node[type="sm"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#6e58d1 #4a2ec6 #3b259e',
				},
			},
			{
				selector: 'node[type="run-target"], node[type="sm"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#6e58d1 #4a2ec6 #3b259e',
				},
			},
			{
				selector: 'node[type="sync-project"], node[type="md"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ffafff #ff79ff #cc61cc',
					height: 80,
					width: 80,
				},
			},
			{
				selector: 'node[type="install-deps"], node[type="lg"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#afe6f2 #79d5e9 #61aaba',
					height: 100,
					width: 100,
				},
			},
			{
				selector: 'node[type="setup-toolchain"], node[type="xl"]',
				style: {
					// @ts-expect-error Types incorrect
					'background-gradient-stop-colors': '#ff9da6 #ff5b6b #cc4956',
					height: 120,
					width: 120,
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
