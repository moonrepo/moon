import { ProtoTool } from '../../../data/proto-tools';
import ToolCard from './ToolCard';

export interface ToolCardsProps {
	tools: Record<string, ProtoTool | ProtoTool[]>;
}

export default function ToolCards(props: ToolCardsProps) {
	return (
		<div className="grid grid-cols-2 gap-2">
			{Object.entries(props.tools).map(([id, entry]) => {
				const showAuthor = Array.isArray(entry);
				const tools = Array.isArray(entry) ? entry : [entry];

				return (
					<>
						{tools.map((tool) => (
							<div key={id} id={id}>
								<ToolCard id={id} tool={tool} showAuthor={showAuthor} />
							</div>
						))}
					</>
				);
			})}
		</div>
	);
}
