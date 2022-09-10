import type { Platform } from './common';

export type DependencyScope = 'development' | 'peer' | 'production';

export interface DependencyConfig {
	id: string;
	scope: DependencyScope;
	via: string | null;
}

export type TaskMergeStrategy = 'append' | 'prepend' | 'replace';

export type TaskOutputStyle = 'buffer-only-failure' | 'buffer' | 'hash' | 'none' | 'stream';

export interface TaskOptionsConfig {
	cache: boolean | null;
	envFile: boolean | string | null;
	mergeArgs: TaskMergeStrategy | null;
	mergeDeps: TaskMergeStrategy | null;
	mergeEnv: TaskMergeStrategy | null;
	mergeInputs: TaskMergeStrategy | null;
	mergeOutputs: TaskMergeStrategy | null;
	outputStyle: TaskOutputStyle | null;
	retryCount: number | null;
	runDepsInParallel: boolean | null;
	runInCI: boolean | null;
	runFromWorkspaceRoot: boolean | null;
}

export interface TaskConfig {
	command: string[] | string | null;
	args: string[] | string | null;
	deps: string[] | null;
	env: Record<string, string> | null;
	inputs: string[] | null;
	local: boolean;
	outputs: string[] | null;
	options: TaskOptionsConfig;
	type: Platform;
}

export type ProjectLanguage = 'bash' | 'batch' | 'javascript' | 'typescript' | 'unknown';

export type ProjectType = 'application' | 'library' | 'tool' | 'unknown';

export interface ProjectMetadataConfig {
	name: string;
	description: string;
	owner: string;
	maintainers: string[];
	channel: string;
}

export interface ProjectWorkspaceConfig {
	inheritedTasks: {
		exclude: string[] | null;
		include: string[] | null;
		rename: Record<string, string> | null;
	};
	typescript: boolean;
}

export interface ProjectConfig {
	dependsOn: (DependencyConfig | string)[];
	fileGroups: Record<string, string[]>;
	language: ProjectLanguage;
	project: ProjectMetadataConfig | null;
	tasks: Record<string, TaskConfig>;
	type: ProjectType;
	workspace: ProjectWorkspaceConfig;
}

export interface GlobalProjectConfig {
	extends: string | null;
	fileGroups: Record<string, string[]>;
	tasks: Record<string, TaskConfig>;
}
