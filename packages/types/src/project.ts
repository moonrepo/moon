import type { Id } from './common';
import type {
	DependencyScope,
	LanguageType,
	LayerType,
	ProjectConfig,
	ProjectDependencyConfig,
	StackType,
} from './project-config';
import type {
	InheritedTasksConfig,
	Input,
	Output,
	PartialInheritedTasksConfig,
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

export interface FileGroup {
	env: string[];
	files: string[];
	globs: string[];
	id: Id;
}

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
	localOnly?: boolean;
	rootLevel?: boolean;
}

export interface TaskFileInput {
	content?: string | null;
	optional?: boolean;
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
	toolchains: Id[];
}

export interface Project {
	aliases?: string[] | null;
	config: ProjectConfig;
	dependencies?: ProjectDependencyConfig[];
	fileGroups?: Record<string, FileGroup>;
	id: Id;
	inherited?: {
		order: string[];
		config: InheritedTasksConfig;
		layers: Record<string, PartialInheritedTasksConfig>;
		taskLayers: Record<string, string[]>;
	} | null;
	language: LanguageType;
	layer: LayerType;
	root: string;
	source: string;
	stack: StackType;
	tasks?: Record<Id, Task>;
	taskTargets?: string[];
	toolchains?: Id[];
}

export interface ProjectFragment {
	alias?: string | null;
	dependencyScope?: DependencyScope | null;
	id: Id;
	source: string;
	toolchains?: Id[];
}

export interface ProjectGraphInner {
	nodes: Project[];
	node_holes: string[];
	edge_property: 'directed';
	edges: [number, number, DependencyScope][];
}

export interface ProjectGraph {
	graph: ProjectGraphInner;
}

export interface TaskGraphInner {
	nodes: Task[];
	node_holes: string[];
	edge_property: 'directed';
	edges: [number, number, TaskDependencyType][];
}

export interface TaskGraph {
	graph: TaskGraphInner;
}
