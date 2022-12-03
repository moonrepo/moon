import { useEffect, useRef } from 'preact/hooks';
import { render } from '../helpers/render';
import type { GraphInfo } from '../helpers/types';

export const Graph = () => {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			render(graphRef.current, JSON.parse(window.GRAPH_DATA) as GraphInfo);
		}
	}, []);

	return <div ref={graphRef} style={{ height: '80vh', width: '100%' }} />;
};
