import type { ActionNode, Project, Task } from '@moonrepo/types';

export type GraphInfo = GraphInfoV1 | GraphInfoV2;

// v1
export interface GraphNodeV1 {
	id: number;
	label: string;
}

export interface GraphEdgeV1 {
	id: string;
	source: number;
	target: number;
	label: string;
}

export interface GraphInfoV1 {
	nodes: GraphNodeV1[];
	edges: GraphEdgeV1[];
}

// v2
export type GraphNodeV2 = ActionNode | Project | Task;
export type GraphEdgeV2 = [number, number, string];

export interface GraphInfoV2 {
	graph: {
		nodes: GraphNodeV2[];
		edges: GraphEdgeV2[];
	};
}
