import { useEffect, useRef } from 'preact/hooks';
import { render } from '../helpers/render';
import type { GraphInfo } from '../helpers/types';

export interface GraphProps {
	layout: string;
}

export function Graph({ layout }: GraphProps) {
	const graphRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (graphRef.current) {
			render(graphRef.current, JSON.parse(window.GRAPH_DATA) as GraphInfo, layout);
		}
	}, [layout]);

	return <div id="graph" ref={graphRef} style={{ height: '80vh', width: '100%' }} />;
}
