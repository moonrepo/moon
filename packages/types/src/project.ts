import type { DependencyConfig, LanguageType, ProjectConfig, ProjectType } from './project-config';
import type {
	InheritedTasksConfig,
	PlatformType,
	TaskMergeStrategy,
	TaskOutputStyle,
	TaskType,
} from './tasks-config';

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
	shell: boolean;
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
	outputGlobs: string[];
	outputPaths: string[];
	platform: PlatformType;
	target: string;
	type: TaskType;
}

export interface Project {
	alias: string | null;
	config: ProjectConfig;
	dependencies: Record<string, DependencyConfig>;
	fileGroups: Record<string, FileGroup>;
	id: string;
	inheritedConfig: InheritedTasksConfig;
	language: LanguageType;
	root: string;
	source: string;
	tasks: Record<string, Task>;
	type: ProjectType;
}
