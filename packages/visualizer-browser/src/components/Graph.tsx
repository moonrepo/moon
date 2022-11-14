import { useEffect, useRef } from 'preact/compat';
import { render, WorkspaceInfo } from '@moonrepo/visualizer-core';

export const Graph = () => {
	const graphRef = useRef<HTMLDivElement>(null);

	const drawGraph = () => {
		if (graphRef.current) {
			// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
			const data: WorkspaceInfo = JSON.parse(window.GRAPH_DATA);
			return render(graphRef.current, data);
		}
		return null;
	};

	useEffect(() => void drawGraph(), []);

	return (
		<>
			<h2>Graph Test</h2>
			<div ref={graphRef} style={{ height: '80vh', width: '100%' }} />
		</>
	);
};
