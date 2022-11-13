export type WorkspaceNode = {
    id: number,
    label: string,
}

export type WorkspaceEdge = {
    id: string,
    source: number,
    target: number,
}

export type WorkspaceInfo = {
    nodes: WorkspaceNode[],
    edges: WorkspaceEdge[]
}
