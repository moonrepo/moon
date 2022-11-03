import { useCallback } from 'react';
import ReactFlow, {
	Node,
	useNodesState,
	useEdgesState,
	addEdge,
	Connection,
	Edge,
} from 'reactflow';
import 'reactflow/dist/style.css';
import './Flow.css';

const initialNodes: Node[] = [
	{
		id: '1',
		type: 'input',
		data: { label: 'Node 1' },
		position: { x: 250, y: 5 },
	},
	{
		id: '2',
		data: { label: 'Node 2' },
		position: { x: 100, y: 100 },
	},
	{
		id: '3',
		data: { label: 'Node 3' },
		position: { x: 400, y: 100 },
	},
	{
		id: '4',
		data: { label: 'Node 4' },
		position: { x: 400, y: 200 },
	},
];

const initialEdges: Edge[] = [
	{ id: 'e1-2', source: '1', target: '2', animated: true },
	{ id: 'e1-3', source: '1', target: '3', animated: true },
];

function Flow() {
	const [nodes, _, onNodesChange] = useNodesState(initialNodes);
	const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
	const onConnect = useCallback(
		(params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)),
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
}

export default Flow;
