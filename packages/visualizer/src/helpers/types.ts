export interface GraphNode {
	id: number;
	label: string;
}

export interface GraphEdge {
	id: string;
	source: number;
	target: number;
}

export interface GraphInfo {
	nodes: GraphNode[];
	edges: GraphEdge[];
}
