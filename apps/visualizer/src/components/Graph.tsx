import React, { useEffect, useRef } from 'react';
import cytoscape from 'cytoscape';
import { useQuery } from '@tanstack/react-query';
import { client } from '../lib/graphql/client';
import { ProjectInformation } from '../lib/graphql/queries';

export const Graph = () => {
	const { data, isLoading } = useQuery(['projectInformation'], () =>
		client.request(ProjectInformation),
	);
	const graphRef = useRef<HTMLDivElement>(null);

	const drawGraph = () => {
		const nodes =
			data?.workspaceInfo.nodes.map((n) => ({
				data: { id: n.id.toString(), label: n.label },
			})) ?? [];
		const edges =
			data?.workspaceInfo.edges.map((e) => ({
				data: { id: e.id.toString(), source: e.source.toString(), target: e.target.toString() },
			})) ?? [];
		const cy = cytoscape({
			container: graphRef.current,
			elements: { edges, nodes },
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
