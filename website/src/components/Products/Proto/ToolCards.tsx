import { useEffect, useState } from 'react';

import { getAuthorName, loadToolsData, ProtoTool, ProtoToolType } from '../../../data/proto-tools';
import ToolCard from './ToolCard';

export interface ToolCardsProps {
	data: ProtoToolType;
}

export default function ToolCards(props: ToolCardsProps) {
	const [tools, setTools] = useState<ProtoTool[]>([]);
	const isBuiltIn = props.data === 'built-in';

	useEffect(() => {
		// oxlint-disable-next-line promise/prefer-await-to-then
		loadToolsData(props.data).then(setTools).catch(console.error);
	}, [props.data]);

	return (
		<div className="grid grid-cols-2 gap-2">
			{tools.map((tool, index) => {
				const id = `${tool.id}-${isBuiltIn ? 'native' : getAuthorName(tool.author)}-${index}`;

				return (
					<div key={id} id={id}>
						<ToolCard id={tool.id} tool={tool} type={props.data} />
					</div>
				);
			})}
		</div>
	);
}
