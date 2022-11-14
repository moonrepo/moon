import cytoscape from 'cytoscape';
import { WorkspaceInfo } from './types';

export * from './types';

export const render = (element: HTMLElement, data: WorkspaceInfo) => {
	const nodes = data.nodes.map((n) => ({
		data: { id: n.id.toString(), label: n.label },
	}));
	const edges = data.edges.map((e) => ({
		data: { id: e.id.toString(), source: e.source.toString(), target: e.target.toString() },
	}));
	const cy = cytoscape({
		container: element,
		elements: { edges, nodes },
		style: [{ selector: 'node', style: { label: 'data(label)' } }],
	});
	return cy;
};
