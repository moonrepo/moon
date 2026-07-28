import type { Action } from './pipeline';
import type { Project, ProjectFragment } from './project';
import type { Task, TaskFragment } from './task';
import type { TemplateVariable } from './template-config';

export interface GetChangedFilesTool {
	base?: string;
	head?: string;
	remote?: boolean;
}

export interface GetChangedFilesToolResponse {
	files: string[];
}

export interface GetProjectTool {
	id: string;
	includeDependencies?: boolean;
}

export interface GetProjectToolResponse {
	project: Project;
	projectDependencies?: Project[];
}

// oxlint-disable-next-line @typescript-eslint/no-empty-interface
export interface GetProjectsTool {}

export interface GetProjectsToolResponse {
	projects: ProjectFragment[];
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
	tasks: TaskFragment[];
}

export interface GetTemplateTool {
	id: string;
}

export interface GetTemplateToolResponse {
	id: string;
	title: string;
	description: string;
	destination?: string;
	extends?: string[];
	variables: Record<string, TemplateVariable>;
}

export interface GetTemplatesTool {
	filter?: string;
}

export interface GetTemplatesToolResponse {
	templates: TemplateSummary[];
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

export interface TemplateSummary {
	id: string;
	title: string;
	description: string;
}
