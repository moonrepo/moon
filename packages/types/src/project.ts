import type { Platform } from './common';
import type {
	DependencyConfig,
	ProjectConfig,
	ProjectLanguage,
	ProjectType,
	TaskMergeStrategy,
	TaskOutputStyle,
} from './project-config';

export type TaskType = 'build' | 'run' | 'test';

export interface FileGroup {
	files: string[];
	id: string;
}

export interface TaskOptions {
	affectedFiles: 'args' | 'both' | 'env';
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
	id: string;
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

export interface ProjectDependency extends DependencyConfig {
	source: 'explicit' | 'implicit';
}

export interface Project {
	alias: string | null;
	config: ProjectConfig;
	dependencies: Record<string, ProjectDependency>;
	fileGroups: Record<string, FileGroup>;
	id: string;
	language: ProjectLanguage;
	root: string;
	source: string;
	tasks: Record<string, Task>;
	type: ProjectType;
}
