import React, { useEffect, useRef } from 'react';
import cytoscape, { ElementDefinition } from 'cytoscape';
import { useQuery } from '@tanstack/react-query';
import { client } from '../lib/graphql/client';
import { ProjectInformation } from '../lib/graphql/queries';

export const Graph = () => {
	const { data, isLoading } = useQuery(['projectInformation'], () =>
		client.request(ProjectInformation),
	);
	const graphRef = useRef<HTMLDivElement>(null);

	const drawGraph = () => {
		const elements: ElementDefinition[] = [];
		data?.workspaceInfo.nodes.forEach((project) => {
			elements.push({
				data: { id: project.id.toString(), label: project.label },
				group: 'nodes',
			});
		});
		data?.workspaceInfo.edges.forEach((edge) => {
			elements.push({
				data: { id: edge.id, source: edge.source, target: edge.target },
				group: 'edges',
			});
		});
		const cy = cytoscape({
			container: graphRef.current,
			elements,
			style: [{ selector: 'node', style: { label: 'data(label)' } }],
		});
		return cy;
	};

	useEffect(() => void drawGraph(), [isLoading]);

	return (
		<>
			<h2>Graph Test</h2>
			<div ref={graphRef} style={{ height: '80vh', width: '100%' }} />
		</>
	);
};
