import React, { useEffect, useRef } from 'react';
import { renderGraph } from '../../utils/renderGraph';

export default function ActionGraph() {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			renderGraph(graphRef.current, {
				edges: [
					{
						data: {
							source: 'node-toolchain',
							target: 'sync-workspace',
						},
					},
					{
						data: {
							source: 'node-env',
							target: 'node-toolchain',
						},
					},
					{
						data: {
							source: 'node-deps',
							target: 'node-env',
						},
					},
					{
						data: {
							source: 'sync-project',
							target: 'sync-workspace',
						},
					},
					{
						data: {
							source: 'sync-project',
							target: 'sync-workspace',
						},
					},
					{
						data: {
							source: 'target-clean',
							target: 'sync-project',
						},
					},
					{
						data: {
							source: 'target-build',
							target: 'sync-project',
						},
					},
					{
						data: {
							source: 'target-build',
							target: 'node-deps',
						},
					},
					{
						data: {
							source: 'target-build',
							target: 'target-clean',
						},
					},
					{
						data: {
							source: 'target-package',
							target: 'target-build',
						},
					},
					{
						data: {
							source: 'target-package',
							target: 'node-deps',
						},
					},
					{
						data: {
							source: 'target-package',
							target: 'sync-project',
						},
					},
				],
				nodes: [
					{
						data: {
							id: 'sync-workspace',
							label: 'SyncWorkspace',
						},
					},
					// Toolchain
					{
						data: {
							id: 'node-toolchain',
							label: 'SetupToolchain(node:18.0.0)',
							type: 'xl',
						},
					},
					// Setup env
					{
						data: {
							id: 'node-env',
							label: 'SetupEnvironment(node)',
							type: 'setup-environment',
						},
					},
					// Install deps
					{
						data: {
							id: 'node-deps',
							label: 'InstallWorkspaceDeps(node)',
							type: 'lg',
						},
					},
					// Sync project
					{
						data: {
							id: 'sync-project',
							label: 'SyncProject(example)',
							type: 'md',
						},
					},
					// Run target
					{
						data: {
							id: 'target-clean',
							label: 'RunTask(example:clean)',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-build',
							label: 'RunTask(example:build)',
							type: 'sm',
						},
					},
					{
						data: {
							id: 'target-package',
							label: 'RunTask(example:package)',
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
