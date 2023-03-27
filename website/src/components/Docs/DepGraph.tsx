import React, { useEffect, useRef } from 'react';
import { renderGraph } from '../../utils/renderGraph';

export default function DepGraph() {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			renderGraph(graphRef.current, {
				edges: [
					{
						data: {
							source: 'node-tool',
							target: 'node-deps',
						},
					},
					{
						data: {
							source: 'system-tool',
							target: 'system-deps',
						},
					},
					{
						data: {
							source: 'node-tool',
							target: 'node-sync',
						},
					},
					{
						data: {
							source: 'system-tool',
							target: 'system-sync',
						},
					},
					{
						data: {
							source: 'system-sync',
							target: 'target-clean',
						},
					},
					{
						data: {
							source: 'system-deps',
							target: 'target-clean',
						},
					},
					{
						data: {
							source: 'node-sync',
							target: 'target-build',
						},
					},
					{
						data: {
							source: 'node-deps',
							target: 'target-build',
						},
					},
					{
						data: {
							source: 'target-clean',
							target: 'target-build',
						},
					},
					{
						data: {
							source: 'target-build',
							target: 'target-package',
						},
					},
				],
				nodes: [
					// Toolchain
					{
						data: {
							id: 'node-tool',
							label: 'SetupNodeTool(18.0.0)',
							type: 'xl',
						},
					},
					{
						data: {
							id: 'system-tool',
							label: 'SetupSystemTool',
							type: 'xl',
						},
					},
					// Install deps
					{
						data: {
							id: 'node-deps',
							label: 'InstallNodeDeps(18.0.0)',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'system-deps',
							label: 'InstallSystemDepsInProject(example)',
							type: 'lg',
						},
					},
					// Sync project
					{
						data: {
							id: 'node-sync',
							label: 'SyncNodeProject(example)',
							type: 'md',
						},
					},
					{
						data: {
							id: 'system-sync',
							label: 'SyncSystemProject(example)',
							type: 'md',
						},
					},
					// Run target
					{
						data: {
							id: 'target-clean',
							label: 'RunTarget(example:clean)',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-build',
							label: 'RunTarget(example:build)',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-package',
							label: 'RunTarget(example:package)',
							type: 'sm',
						},
					},
				],
			});
		}
	}, []);

	return (
		<div
			id="dep-graph"
			ref={graphRef}
			className="p-1 mb-2 rounded bg-slate-800"
			style={{ height: '550px', width: '100%' }}
		/>
	);
}
