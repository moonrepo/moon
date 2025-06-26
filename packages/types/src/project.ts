import type {
	DependencyConfig,
	DependencyScope,
	DependencyType,
	LanguageType,
	LayerType,
	ProjectConfig,
	StackType,
} from './project-config';
import type {
	InheritedTasksConfig,
	PartialInheritedTasksConfig,
	PlatformType,
	TaskDependencyConfig,
	TaskMergeStrategy,
	TaskOperatingSystem,
	TaskOutputStyle,
	TaskPriority,
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
	affectedPassInputs: boolean;
	allowFailure: boolean;
	cache: boolean;
	cacheKey: string | null;
	cacheLifetime: string | null;
	envFiles: string[] | null;
	inferInputs: boolean;
	internal: boolean;
	interactive: boolean;
	mergeArgs: TaskMergeStrategy;
	mergeDeps: TaskMergeStrategy;
	mergeEnv: TaskMergeStrategy;
	mergeInputs: TaskMergeStrategy;
	mergeOutputs: TaskMergeStrategy;
	mutex: string | null;
	os: TaskOperatingSystem[] | null;
	outputStyle: TaskOutputStyle | null;
	persistent: boolean;
	priority: TaskPriority;
	retryCount: number;
	runDepsInParallel: boolean;
	runInCI: boolean;
	runFromWorkspaceRoot: boolean;
	shell: boolean | null;
	timeout: number | null;
	unixShell: TaskUnixShell | null;
	windowsShell: TaskWindowsShell | null;
}

export interface TaskState {
	defaultInputs: boolean;
	emptyInputs: boolean;
	expanded: boolean;
	localOnly: boolean;
	rootLevel: boolean;
}

export interface Task {
	args: string[];
	command: string;
	deps: TaskDependencyConfig[];
	description: string | null;
	env: Record<string, string>;
	id: string;
	inputs: string[];
	inputEnv: string[];
	inputFiles: string[];
	inputGlobs: string[];
	options: TaskOptions;
	outputs: string[];
	outputFiles: string[];
	outputGlobs: string[];
	platform: PlatformType;
	preset: 'server' | 'watcher' | null;
	script: string | null;
	state: TaskState;
	target: string;
	toolchains: string[];
	type: TaskType;
}

export interface TaskFragment {
	target: string;
	toolchains: string[];
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
	layer: LayerType;
	platform: PlatformType;
	root: string;
	source: string;
	stack: StackType;
	tasks: Record<string, Task>;
	taskTargets: string[];
	toolchains: string[];
}

export interface ProjectFragment {
	alias: string | null;
	dependencyScope: DependencyScope;
	id: string;
	source: string;
	toolchains: string[];
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
	edges: [number, number, DependencyType][];
}

export interface TaskGraph {
	graph: TaskGraphInner;
}

export interface WorkspaceGraph {
	projects_by_tag: Record<string, string[]>;
	project_data: Record<string, { alias: string; node_index: number; source: string }>;
	project_graph: ProjectGraphInner;
	renamed_project_ids: Record<string, string>;
	repo_type: 'monorepo-with-root' | 'monorepo' | 'polyrepo';
	root_project_id: string | null;
	task_data: Record<string, { node_index: number }>;
	task_graph: TaskGraphInner;
}
