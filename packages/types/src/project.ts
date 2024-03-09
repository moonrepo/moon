import type {
	DependencyConfig,
	DependencyScope,
	LanguageType,
	ProjectConfig,
	ProjectType,
} from './project-config';
import type {
	InheritedTasksConfig,
	PartialInheritedTasksConfig,
	PlatformType,
	TaskDependencyConfig,
	TaskMergeStrategy,
	TaskOutputStyle,
	TaskType,
	TaskUnixShell,
	TaskWindowsShell,
} from './tasks-config';

export interface FileGroup {
	env: string[];
	files: string[];
	globs: string[];
	id: string;
}

export interface TaskOptions {
	affectedFiles: boolean | 'args' | 'env' | null;
	allowFailure: boolean;
	cache: boolean;
	envFiles: string[] | null;
	interactive: boolean;
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
	unixShell: TaskUnixShell | null;
	windowsShell: TaskWindowsShell | null;
}

export interface Task {
	args: string[];
	command: string;
	deps: TaskDependencyConfig[];
	env: Record<string, string>;
	id: string;
	inputs: string[];
	inputFiles: string[];
	inputGlobs: string[];
	inputVars: string[];
	metadata: Record<string, unknown>;
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
	dependencies: DependencyConfig[];
	fileGroups: Record<string, FileGroup>;
	id: string;
	inherited: {
		order: string[];
		config: InheritedTasksConfig;
		layers: Record<string, PartialInheritedTasksConfig>;
		taskLayers: Record<string, string[]>;
	};
	language: LanguageType;
	platform: PlatformType;
	root: string;
	source: string;
	tasks: Record<string, Task>;
	type: ProjectType;
}

export interface ProjectGraphInner {
	nodes: Project[];
	node_holes: string[];
	edge_property: 'directed';
	edges: [number, number, DependencyScope][];
}

export interface PartialProjectGraph {
	aliases: Record<string, string>;
	graph: ProjectGraphInner;
	nodes: Record<string, number>;
	root_id: string | null;
	sources: Record<string, string>;
}

export interface ProjectGraph {
	graph: ProjectGraphInner;
	projects: Record<string, Project>;
}
