import type { GraphContainer, Id } from './common';
import type {
	DependencyScope,
	LanguageType,
	LayerType,
	ProjectConfig,
	ProjectDependencyConfig,
	StackType,
} from './project-config';
import type { Task } from './task';
import type { InheritedTasksConfig, PartialInheritedTasksConfig } from './tasks-config';

export interface FileGroup {
	env: string[];
	files: string[];
	globs: string[];
	id: Id;
}

export interface ProjectAlias {
	alias: string;
	plugin: Id;
}

export interface Project {
	aliases?: ProjectAlias[] | null;
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

export type ProjectGraph = GraphContainer<Project, DependencyScope>;
