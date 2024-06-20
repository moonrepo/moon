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
							source: 'sync-workspace',
							target: 'node-toolchain',
						},
					},
					{
						data: {
							source: 'sync-workspace',
							target: 'system-toolchain',
						},
					},
					{
						data: {
							source: 'node-toolchain',
							target: 'node-deps',
						},
					},
					{
						data: {
							source: 'system-toolchain',
							target: 'system-deps',
						},
					},
					{
						data: {
							source: 'node-toolchain',
							target: 'node-sync',
						},
					},
					{
						data: {
							source: 'system-toolchain',
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
							label: 'SetupNodeToolchain(18.0.0)',
							type: 'xl',
						},
					},
					{
						data: {
							id: 'system-toolchain',
							label: 'SetupSystemToolchain',
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
