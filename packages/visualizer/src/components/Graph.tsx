import type { WorkspaceInfo } from '../lib/types';
import React, { useEffect, useRef } from 'react';
import cytoscape from 'cytoscape';

export const Graph = () => {
	const graphRef = useRef<HTMLDivElement>(null);

	const drawGraph = () => {
		const data: WorkspaceInfo = JSON.parse(window.GRAPH_DATA);
		const nodes = data.nodes.map((n) => ({
			data: { id: n.id.toString(), label: n.label },
		}));
		const edges = data.edges.map((e) => ({
			data: { id: e.id.toString(), source: e.source.toString(), target: e.target.toString() },
		}));
		const cy = cytoscape({
			container: graphRef.current,
			elements: { edges, nodes },
			style: [{ selector: 'node', style: { label: 'data(label)' } }],
		});
		return cy;
	};

	useEffect(() => void drawGraph(), []);

	return (
		<>
			<h2>Graph Test</h2>
			<div ref={graphRef} style={{ height: '80vh', width: '100%' }} />
		</>
	);
};
