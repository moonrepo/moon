import { useEffect, useRef } from 'preact/hooks';
import { render } from '../lib/render';
import type { GraphInfo } from '../lib/types';

export const Graph = () => {
	const graphRef = useRef<HTMLDivElement>(null);

	const drawGraph = () => {
		if (graphRef.current)
			return render(graphRef.current, JSON.parse(window.GRAPH_DATA) as GraphInfo);
		return null;
	};

	useEffect(() => void drawGraph(), []);

	return <div ref={graphRef} style={{ height: '80vh', width: '100%' }} />;
};
