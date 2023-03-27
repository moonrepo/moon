import React, { useEffect, useRef } from 'react';
import { renderGraph } from '../../utils/renderGraph';

export default function LangGraph() {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			renderGraph(graphRef.current, {
				edges: [
					// Node.js
					{
						data: {
							source: 'node',
							target: 'node-lang',
						},
					},
					{
						data: {
							source: 'node',
							target: 'node-platform',
						},
					},
					{
						data: {
							source: 'node',
							target: 'node-tool',
						},
					},
					{
						data: {
							source: 'node-platform',
							target: 'node-lang',
						},
					},
					{
						data: {
							source: 'node-tool',
							target: 'node-lang',
						},
					},
					{
						data: {
							source: 'node-platform',
							target: 'node-tool',
						},
					},
					// Deno
					{
						data: {
							source: 'deno',
							target: 'deno-lang',
						},
					},
					{
						data: {
							source: 'deno',
							target: 'deno-platform',
						},
					},
					{
						data: {
							source: 'deno-platform',
							target: 'deno-lang',
						},
					},
					// Bun
					{
						data: {
							source: 'bun',
							target: 'bun-lang',
						},
					},
					// All
					{
						data: {
							source: 'moon',
							target: 'system',
						},
					},
					{
						data: {
							source: 'moon',
							target: 'go',
						},
					},
					{
						data: {
							source: 'moon',
							target: 'bun',
						},
					},
					{
						data: {
							source: 'moon',
							target: 'deno',
						},
					},
					{
						data: {
							source: 'moon',
							target: 'node',
						},
					},
				],
				nodes: [
					// Base
					{
						data: {
							id: 'moon',
							label: 'moon',
							type: 'xl',
						},
					},
					// System
					{
						data: {
							id: 'system',
							label: 'System (Fallback)',
							type: 'md',
						},
					},
					// Node.js
					{
						data: {
							id: 'node',
							label: 'Node (Tier 3)',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'node-lang',
							label: 'Language (1)',
						},
					},
					{
						data: {
							id: 'node-platform',
							label: 'Platform (2)',
						},
					},
					{
						data: {
							id: 'node-tool',
							label: 'Toolchain (3)',
						},
					},
					// Deno
					{
						data: {
							id: 'deno',
							label: 'Deno (Tier 2)',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'deno-lang',
							label: 'Language (1)',
						},
					},
					{
						data: {
							id: 'deno-platform',
							label: 'Platform (2)',
						},
					},
					// Bun
					{
						data: {
							id: 'bun',
							label: 'Bun (Tier 1)',
							type: 'lg',
						},
					},
					{
						data: {
							id: 'bun-lang',
							label: 'Language (1)',
						},
					},
					// Go
					{
						data: {
							id: 'go',
							label: 'Go (Tier 0)',
							type: 'lg',
						},
					},
				],
			});
		}
	}, []);

	return (
		<div
			id="lang-graph"
			ref={graphRef}
			className="p-1 mb-2 rounded bg-slate-800"
			style={{ height: '600px', width: '100%' }}
		/>
	);
}
