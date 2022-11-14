export interface WorkspaceNode {
	id: number;
	label: string;
}

export interface WorkspaceEdge {
	id: string;
	source: number;
	target: number;
}

export interface WorkspaceInfo {
	nodes: WorkspaceNode[];
	edges: WorkspaceEdge[];
}
