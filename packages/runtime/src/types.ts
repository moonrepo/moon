import { Path } from '@boost/common';

export interface FileGroup {
	files: string;
	id: string;
}

// Keep in sync with crates/project/src/task.rs
export type TaskMergeStrategy = 'append' | 'prepend' | 'replace';

export interface TaskOptions {
	mergeArgs: TaskMergeStrategy;
	mergeDeps: TaskMergeStrategy;
	mergeEnv: TaskMergeStrategy;
	mergeInputs: TaskMergeStrategy;
	mergeOutputs: TaskMergeStrategy;
	retryCount: number;
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
	options: TaskOptions;
	outputs: string[];
	outputPaths: string[];
	target: string;
	type: 'node' | 'system';
}

// Keep in sync with crates/project/src/project.rs
export interface Project {
	config: object;
	fileGroups: Record<string, FileGroup>;
	id: string;
	root: Path;
	source: string;
	tasks: Record<string, Task>;
}
