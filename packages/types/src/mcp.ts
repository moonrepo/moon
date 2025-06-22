import type { Action } from './pipeline';
import type { Project, Task } from './project';

export interface GetProjectTool {
	id: string;
	includeDependencies?: boolean;
}

export interface GetProjectToolResponse {
	project: Project;
	projectDependencies?: Project[];
}

export interface GetProjectsTool {
	includeTasks?: boolean;
}

export interface GetProjectsToolResponse {
	projects: Project[];
}

export interface GetTaskTool {
	target: string;
	includeDependencies?: boolean;
}

export interface GetTaskToolResponse {
	task: Task;
	taskDependencies?: Task[];
}

export interface GetTasksTool {
	includeInternal?: boolean;
}

export interface GetTasksToolResponse {
	tasks: Task[];
}

export interface GetTouchedFilesTool {
	base?: string;
	head?: string;
	remote?: boolean;
}

export interface GetTouchedFilesToolResponse {
	files: string[];
}

export interface SyncProjectsTool {
	ids: string[];
}

export interface SyncProjectsToolResponse {
	actions: Action[];
	synced: boolean;
}

export interface SyncWorkspaceToolResponse {
	actions: Action[];
	synced: boolean;
}
