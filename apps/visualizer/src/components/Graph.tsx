import React, { Fragment, useEffect, useRef } from 'react';
import cytoscape from 'cytoscape';

export const Graph = () => {
	const graphRef = useRef(null);

	const drawGraph = () => {
		const cy = cytoscape({
			container: graphRef.current,
			elements: [
				{
					data: { id: 'n0' },
					group: 'nodes',
				},
				{
					data: { id: 'n1' },
					group: 'nodes',
				},
				{
					data: { id: 'n2' },
					group: 'nodes',
				},
				{
					data: { id: 'e0', source: 'n0', target: 'n1' },
					group: 'edges',
				},
				{
					data: { id: 'e1', source: 'n2', target: 'n1' },
					group: 'edges',
				},
				{
					data: { id: 'e2', source: 'n2', target: 'n2' },
					group: 'edges',
				},
			],
			style: [
				{
					selector: 'node',
					style: { label: 'data(id)' },
				},
			],
		});
		return cy;
	};

	useEffect(() => void drawGraph(), []);

	return (
		<Fragment>
			<h2>Graph Test</h2>
			<div ref={graphRef} style={{ height: '80vh', width: '100%' }}></div>
		</Fragment>
	);
};
