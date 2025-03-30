import { useEffect, useState } from 'react';
import { getAuthorName, loadToolsData, ProtoTool } from '../../../data/proto-tools';
import ToolCard from './ToolCard';

export interface ToolCardsProps {
	data: 'built-in' | 'third-party';
}

export default function ToolCards(props: ToolCardsProps) {
	const [tools, setTools] = useState<ProtoTool[]>([]);
	const isThirdParty = props.data === 'third-party';

	useEffect(() => {
		// eslint-disable-next-line promise/prefer-await-to-then
		loadToolsData(props.data).then(setTools).catch(console.error);
	}, []);

	return (
		<div className="grid grid-cols-2 gap-2">
			{tools.map((tool, index) => {
				const id = `${tool.id}-${isThirdParty ? getAuthorName(tool.author) : 'native'}-${index}`;

				return (
					<div key={id} id={id}>
						<ToolCard id={tool.id} tool={tool} builtin={!isThirdParty} />
					</div>
				);
			})}
		</div>
	);
}
