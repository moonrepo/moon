import type { DependencyConfig, LanguageType, ProjectConfig, ProjectType } from './project-config';
import type {
	InheritedTasksConfig,
	PartialInheritedTasksConfig,
	PlatformType,
	TaskMergeStrategy,
	TaskOutputStyle,
	TaskType,
} from './tasks-config';

export interface InputGroup {
	env: string[];
	files: string[];
	globs: string[];
	id: string;
}

export interface TaskOptions {
	affectedFiles: boolean | 'args' | 'env';
	cache: boolean;
	envFile: string | null;
	mergeArgs: TaskMergeStrategy;
	mergeDeps: TaskMergeStrategy;
	mergeEnv: TaskMergeStrategy;
	mergeInputs: TaskMergeStrategy;
	mergeOutputs: TaskMergeStrategy;
	outputStyle: TaskOutputStyle | null;
	persistent: boolean;
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
	inputFiles: string[];
	inputGlobs: string[];
	inputVars: string[];
	options: TaskOptions;
	outputs: string[];
	outputFiles: string[];
	outputGlobs: string[];
	platform: PlatformType;
	target: string;
	type: TaskType;
}

export interface Project {
	alias: string | null;
	config: ProjectConfig;
	dependencies: Record<string, DependencyConfig>;
	inputGroups: Record<string, InputGroup>;
	id: string;
	inherited: {
		order: string[];
		layers: Record<string, PartialInheritedTasksConfig>;
		config: InheritedTasksConfig;
	};
	language: LanguageType;
	platform: PlatformType;
	root: string;
	source: string;
	tasks: Record<string, Task>;
	type: ProjectType;
}
