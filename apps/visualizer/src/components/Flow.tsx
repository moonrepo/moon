import 'reactflow/dist/style.css';
import './Flow.css';
import React, { useCallback } from 'react';
import ReactFlow, {
	addEdge,
	Connection,
	Edge,
	Node,
	useEdgesState,
	useNodesState,
} from 'reactflow';

const initialNodes: Node[] = [
	{
		data: { label: 'Node 1' },
		id: '1',
		position: { x: 250, y: 5 },
		type: 'input',
	},
	{
		data: { label: 'Node 2' },
		id: '2',
		position: { x: 100, y: 100 },
	},
	{
		data: { label: 'Node 3' },
		id: '3',
		position: { x: 400, y: 100 },
	},
	{
		data: { label: 'Node 4' },
		id: '4',
		position: { x: 400, y: 200 },
	},
];

const initialEdges: Edge[] = [
	{ animated: true, id: 'e1-2', source: '1', target: '2' },
	{ animated: true, id: 'e1-3', source: '1', target: '3' },
];

export const Flow = () => {
	const [nodes, , onNodesChange] = useNodesState(initialNodes);
	const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
	const onConnect = useCallback(
		(params: Connection | Edge) => void setEdges((eds) => addEdge(params, eds)),
		[setEdges],
	);

	return (
		<div className="Flow">
			<ReactFlow
				nodes={nodes}
				onNodesChange={onNodesChange}
				edges={edges}
				onEdgesChange={onEdgesChange}
				onConnect={onConnect}
				fitView
			/>
		</div>
	);
};
