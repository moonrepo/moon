import type { Nullable, Platform } from './common';
import type { NodeConfig } from './toolchain-config';

export type DependencyScope = 'development' | 'peer' | 'production';

export interface DependencyConfig {
	id: string;
	scope: DependencyScope;
	via: string | null;
}

export type TaskMergeStrategy = 'append' | 'prepend' | 'replace';

export type TaskOutputStyle = 'buffer-only-failure' | 'buffer' | 'hash' | 'none' | 'stream';

export interface TaskOptionsConfig {
	affectedFiles: boolean | 'args' | 'env' | null;
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
	platform: Platform;
}

export type ProjectLanguage =
	| 'bash'
	| 'batch'
	| 'go'
	| 'javascript'
	| 'php'
	| 'python'
	| 'ruby'
	| 'rust'
	| 'typescript'
	| 'unknown';

export type ProjectType = 'application' | 'library' | 'tool' | 'unknown';

export interface ProjectMetadataConfig {
	name: string;
	description: string;
	owner: string;
	maintainers: string[];
	channel: string;
}

export type ProjectToolchainNodeConfig = Nullable<Pick<NodeConfig, 'version'>>;

export interface ProjectToolchainConfig {
	node: ProjectToolchainNodeConfig | null;
	typescript: boolean;
}

export interface ProjectWorkspaceConfig {
	inheritedTasks: {
		exclude: string[] | null;
		include: string[] | null;
		rename: Record<string, string> | null;
	};
}

export interface ProjectConfig {
	dependsOn: (DependencyConfig | string)[];
	env: Record<string, string> | null;
	fileGroups: Record<string, string[]>;
	language: ProjectLanguage;
	platform: Platform | null;
	project: ProjectMetadataConfig | null;
	tasks: Record<string, TaskConfig>;
	toolchain: ProjectToolchainConfig;
	type: ProjectType;
	workspace: ProjectWorkspaceConfig;
}

export interface InheritedTasksConfig {
	extends: string | null;
	fileGroups: Record<string, string[]>;
	implicitDeps: string[];
	implicitInputs: string[];
	tasks: Record<string, TaskConfig>;
}
