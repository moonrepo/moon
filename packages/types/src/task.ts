import type { GraphContainer, Id } from './common';
import type {
	Input,
	Output,
	TaskDependencyConfig,
	TaskDependencyType,
	TaskMergeStrategy,
	TaskOperatingSystem,
	TaskOutputStyle,
	TaskPreset,
	TaskPriority,
	TaskType,
	TaskUnixShell,
	TaskWindowsShell,
} from './tasks-config';

export interface TaskOptions {
	affectedFiles?: boolean | 'args' | 'env' | null;
	affectedPassInputs: boolean;
	allowFailure: boolean;
	cache: boolean | 'local' | 'remote';
	cacheKey?: string | null;
	cacheLifetime?: string | null;
	envFiles?: string[] | null;
	inferInputs: boolean;
	internal: boolean;
	interactive: boolean;
	mergeArgs: TaskMergeStrategy;
	mergeDeps: TaskMergeStrategy;
	mergeEnv: TaskMergeStrategy;
	mergeInputs: TaskMergeStrategy;
	mergeOutputs: TaskMergeStrategy;
	mergeToolchains: TaskMergeStrategy;
	mutex?: string | null;
	os?: TaskOperatingSystem[] | null;
	outputStyle?: TaskOutputStyle | null;
	persistent: boolean;
	priority: TaskPriority;
	retryCount: number;
	runDepsInParallel: boolean;
	runInCI: boolean;
	runFromWorkspaceRoot: boolean;
	shell?: boolean | null;
	timeout?: number | null;
	unixShell?: TaskUnixShell | null;
	windowsShell?: TaskWindowsShell | null;
}

export interface TaskState {
	defaultInputs?: boolean;
	emptyInputs?: boolean;
	expanded?: boolean;
	rootLevel?: boolean;
}

export interface TaskFileInput {
	content?: string | null;
	optional?: boolean | null;
}

export interface TaskGlobInput {
	cache?: boolean;
}

export interface TaskFileOutput {
	optional?: boolean;
}

// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface TaskGlobOutput {}

export interface Task {
	args?: string[];
	command: string;
	deps?: TaskDependencyConfig[];
	description?: string | null;
	env?: Record<string, string>;
	id: Id;
	inputs?: Input[];
	inputEnv?: string[];
	inputFiles?: Record<string, TaskFileInput>;
	inputGlobs?: Record<string, TaskGlobInput>;
	options: TaskOptions;
	outputs?: Output[];
	outputFiles?: Record<string, TaskFileOutput>;
	outputGlobs?: Record<string, TaskGlobOutput>;
	preset?: TaskPreset | null;
	script?: string | null;
	state: TaskState;
	target: string;
	toolchains?: Id[];
	type: TaskType;
}

export interface TaskFragment {
	target: string;
	toolchains?: Id[];
}

export type TaskGraph = GraphContainer<Task, TaskDependencyType>;
