export type Platform = 'node' | 'system' | 'unknown';

export interface FileGroup {
	files: string[];
	id: string;
}

export type TaskMergeStrategy = 'append' | 'prepend' | 'replace';

export type TaskOutputStyle = 'buffer-only-failure' | 'buffer' | 'hash' | 'none' | 'stream';

export interface TaskOptions {
	cache: boolean;
	envFile: string | null;
	mergeArgs: TaskMergeStrategy;
	mergeDeps: TaskMergeStrategy;
	mergeEnv: TaskMergeStrategy;
	mergeInputs: TaskMergeStrategy;
	mergeOutputs: TaskMergeStrategy;
	outputStyle: TaskOutputStyle | null;
	retryCount: number;
	runDepsInParallel: boolean;
	runInCI: boolean;
	runFromWorkspaceRoot: boolean;
}

export interface Task {
	args: string[];
	command: string;
	deps: string[];
	env: Record<string, string>;
	inputs: string[];
	inputGlobs: string[];
	inputPaths: string[];
	inputVars: string[];
	options: TaskOptions;
	outputs: string[];
	outputPaths: string[];
	platform: Platform;
	target: string;
}

export interface Project {
	alias: string | null;
	config: ProjectConfig;
	dependencies: DependencyConfig[];
	fileGroups: Record<string, FileGroup>;
	id: string;
	root: string;
	source: string;
	tasks: Record<string, Task>;
}
