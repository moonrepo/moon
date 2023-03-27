import React, { useEffect, useRef } from 'react';
import { renderGraph } from '../../utils/renderGraph';

export default function ProjectGraph() {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			renderGraph(graphRef.current, {
				edges: [
					// Client
					{
						data: {
							source: 'app-client',
							target: 'lib-components',
						},
					},
					{
						data: {
							source: 'app-client',
							target: 'lib-utils',
						},
					},
					{
						data: {
							source: 'app-client',
							target: 'lib-intl',
						},
					},
					// Server
					{
						data: {
							source: 'app-server',
							target: 'lib-utils',
						},
					},
					{
						data: {
							source: 'app-server',
							target: 'lib-intl',
						},
					},
					// Libs
					{
						data: {
							source: 'lib-components',
							target: 'lib-intl',
						},
					},
					{
						data: {
							source: 'lib-components',
							target: 'lib-theme',
						},
					},
					{
						data: {
							source: 'lib-components',
							target: 'lib-utils',
						},
					},
				],
				nodes: [
					// Apps
					{
						data: {
							id: 'app-client',
							label: 'Client app',
							type: 'xl',
						},
					},
					{
						data: {
							id: 'app-server',
							label: 'Server app',
							type: 'xl',
						},
					},
					// Libraries
					{
						data: {
							id: 'lib-components',
							label: 'Components',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'lib-utils',
							label: 'Utils',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'lib-theme',
							label: 'Theme',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'lib-intl',
							label: 'Intl',
							type: 'lg',
						},
					},
				],
			});
		}
	}, []);

	return (
		<div
			id="project-graph"
			ref={graphRef}
			className="p-1 mb-2 rounded bg-slate-800"
			style={{ height: '450px', width: '100%' }}
		/>
	);
}
