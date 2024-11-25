import React, { useEffect, useRef } from 'react';
import { renderGraph } from '../../utils/renderGraph';

export default function TaskGraph() {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			renderGraph(graphRef.current, {
				edges: [
					{
						data: {
							source: 'target-app-build',
							target: 'target-app-clean',
						},
					},
					{
						data: {
							source: 'target-app-build',
							target: 'target-components-build',
						},
					},
					{
						data: {
							source: 'target-app-build',
							target: 'target-types-codegen',
						},
					},
					{
						data: {
							source: 'target-components-build',
							target: 'target-types-codegen',
						},
					},
				],
				nodes: [
					{
						data: {
							id: 'target-app-build',
							label: 'app:build',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-app-clean',
							label: 'app:clean',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-components-build',
							label: 'components:build',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-types-codegen',
							label: 'types:codegen',
							type: 'sm',
						},
					},
				],
			});
		}
	}, []);

	return (
		<div
			id="task-graph"
			ref={graphRef}
			className="p-1 mb-2 rounded bg-slate-800"
			style={{ height: '450px', width: '100%' }}
		/>
	);
}
