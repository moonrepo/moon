export type Id = string;

export type Nullable<T> = { [K in keyof T]: T[K] | null };

export interface Duration {
	secs: number;
	nanos: number;
}

export interface ToolchainSpec {
	id: Id;
	req?: string | null;
}

export type ExtendsFrom = string[] | string;

export interface Graph<Node, Edge> {
	nodes: Node[];
	node_holes: string[];
	edge_property: 'directed';
	edges: [number, number, Edge][];
}

export interface GraphContainer<Node, Edge> {
	graph: Graph<Node, Edge>;
}
