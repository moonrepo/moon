import fs from 'fs';
import { json, Path } from '@boost/common';
import { Project } from './types';

export interface RuntimeContext {
	project: Project;
	target: string;
	workspace: {
		root: Path;
	};
}

// Trigger CI

export async function getContext(): Promise<RuntimeContext> {
	const { env } = process;

	if (!env.MOON_PROJECT_RUNFILE) {
		throw new Error('Attempting to access moon context outside of a run process.');
	}

	const project = json.parse<Project>(await fs.promises.readFile(env.MOON_PROJECT_RUNFILE, 'utf8'));

	return {
		project: {
			...project,
			root: Path.create(project.root),
		},
		target: env.MOON_TARGET!,
		workspace: {
			root: Path.create(env.MOON_WORKSPACE_ROOT ?? process.cwd()),
		},
	};
}
