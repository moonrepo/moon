import type { NxJsonConfiguration as NxJson } from 'nx/src/config/nx-json';
import type { ProjectsConfigurations as WorkspaceJson } from 'nx/src/config/workspace-json-project-json';
import { json, Path } from '@boost/common';
import { loadAndCacheJson } from './helpers';

export async function loadNxJson(root: string): Promise<NxJson> {
	return loadAndCacheJson('nx-json', root, () => {
		const nxJsonPath = new Path(root, 'nx.json');
		const nxJson: NxJson = {};

		if (nxJsonPath.exists()) {
			Object.assign(nxJson, json.load(nxJsonPath));
		}

		return nxJson;
	});
}

export async function loadWorkspaceJson(root: string): Promise<WorkspaceJson> {
	return loadAndCacheJson('nx-json', root, () => {
		const workspaceJsonPath = new Path(root, 'workspace.json');
		const workspaceJson: WorkspaceJson = { projects: {}, version: 2 };

		if (workspaceJsonPath.exists()) {
			Object.assign(workspaceJson, json.load(workspaceJsonPath));
		}

		return workspaceJson;
	});
}
