export interface ProtoTool {
	name: string;
	description: string;
	homepageUrl?: string;
	repositoryUrl: string;
	devicon?: string;

	// Plugin information:
	// https://moonrepo.dev/docs/proto/plugins#enabling-plugins
	locator?: string;
	format: 'toml' | 'wasm';
	id: string;
	author: string | { name: string; email?: string; url?: string };

	// Available global binaries/directories:
	// https://moonrepo.dev/docs/proto/wasm-plugin#locating-binaries
	bins?: string[];
	globalsDirs?: string[];

	// Version detection sources:
	// https://moonrepo.dev/docs/proto/wasm-plugin#detecting-versions
	detectionSources?: {
		file: string;
		label?: string;
		url?: string;
	}[];
}

export function getAuthorName(author: ProtoTool['author']): string {
	return typeof author === 'string' ? author : author.name;
}

export async function loadToolsData(type: 'built-in' | 'third-party'): Promise<ProtoTool[]> {
	const res = await fetch(
		`https://raw.githubusercontent.com/moonrepo/proto/master/registry/data/${type}.json`,
		{ cache: 'default' },
	);

	const data = (await res.json()) as { plugins: ProtoTool[] };

	return data.plugins;
}
