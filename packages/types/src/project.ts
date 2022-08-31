import type { Platform } from './common';
import type { DependencyConfig, ProjectConfig, TaskMergeStrategy, TaskOutputStyle } from './config';

export type TaskType = 'build' | 'run' | 'test';

export interface FileGroup {
	files: string[];
	id: string;
}

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
	type: TaskType;
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
