import { ProtoTool } from '../../../data/proto-tools';
import ToolCard from './ToolCard';

export interface ToolCardsProps {
	tools: Record<string, ProtoTool>;
}

export default function ToolCards(props: ToolCardsProps) {
	return (
		<div className="grid grid-cols-2 gap-2">
			{Object.entries(props.tools).map(([id, tool]) => (
				<div key={id} id={id}>
					<ToolCard id={id} tool={tool} />
				</div>
			))}
		</div>
	);
}
