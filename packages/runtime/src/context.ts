import fs from 'node:fs';
import { json, Path } from '@boost/common';
import { type Project } from '@moonrepo/types';

export interface RuntimeContext {
	project: Project;
	projectRoot: Path;
	target: string;
	workspaceRoot: Path;
}

export async function getContext(): Promise<RuntimeContext> {
	const { env } = process;

	if (!env.MOON_PROJECT_SNAPSHOT) {
		throw new Error('Attempting to access moon context outside of a run process.');
	}

	const project = json.parse<Project>(
		await fs.promises.readFile(env.MOON_PROJECT_SNAPSHOT, 'utf8'),
	);

	return {
		project,
		projectRoot: Path.create(project.root),
		target: env.MOON_TARGET!,
		workspaceRoot: Path.create(env.MOON_WORKSPACE_ROOT ?? process.cwd()),
	};
}
