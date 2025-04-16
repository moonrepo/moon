import type { Duration, Runtime, ToolchainSpec } from './common';
import type {
	Action,
	ActionContext,
	ActionNode,
	ActionNodeRunTask,
	ActionPipelineStatus,
} from './pipeline';
import type { Project } from './project';

export interface ProviderEnvironment {
	baseBranch: string | null;
	baseRevision: string | null;
	branch: string;
	headRevision: string | null;
	id: string;
	provider: string;
	requestId: string | null;
	requestUrl: string | null;
	revision: string;
	url: string | null;
}

export interface WebhookPayload<T extends EventType, E> {
	createdAt: string;
	environment: ProviderEnvironment | null;
	event: E;
	type: T;
	uuid: string;
}

export type EventType =
	// eslint-disable-next-line @typescript-eslint/sort-type-constituents
	| 'action.completed'
	| 'action.started'
	| 'dependencies.installed'
	| 'dependencies.installing'
	| 'pipeline.completed'
	| 'pipeline.started'
	| 'project.synced'
	| 'project.syncing'
	| 'task.ran'
	| 'task.running'
	| 'tool.installed'
	| 'tool.installing'
	| 'toolchain.installed'
	| 'toolchain.installing'
	| 'workspace.synced'
	| 'workspace.syncing'
	// Legacy < 1.26
	| 'action.finished'
	| 'pipeline.finished'
	| 'pipeline.aborted'
	| 'target.ran'
	| 'target.running';

export interface EventActionStarted {
	action: Action;
	node: ActionNode;
}

export type PayloadActionStarted = WebhookPayload<'action.started', EventActionStarted>;

export interface EventActionCompleted {
	action: Action;
	error: string | null;
	node: ActionNode;
}

export type PayloadActionCompleted = WebhookPayload<'action.completed', EventActionCompleted>;

export interface EventDependenciesInstalling {
	project: Project | null;
	runtime: Runtime;
}

export type PayloadDependenciesInstalling = WebhookPayload<
	'dependencies.installing',
	EventDependenciesInstalling
>;

export interface EventDependenciesInstalled {
	error: string | null;
	project: Project | null;
	runtime: Runtime;
}

export type PayloadDependenciesInstalled = WebhookPayload<
	'dependencies.installed',
	EventDependenciesInstalled
>;

export interface EventProjectSyncing {
	project: Project;
}

export type PayloadProjectSyncing = WebhookPayload<'project.syncing', EventProjectSyncing>;

export interface EventProjectSynced {
	error: string | null;
	project: Project;
}

export type PayloadProjectSynced = WebhookPayload<'project.synced', EventProjectSynced>;

export interface EventPipelineStarted {
	actionsCount: number;
	actionNodes: ActionNode[];
	context: ActionContext;
}

export type PayloadPipelineStarted = WebhookPayload<'pipeline.started', EventPipelineStarted>;

export interface EventPipelineCompleted {
	actions: Action[];
	context: ActionContext;
	duration: Duration | null;
	error: string | null;
	status: ActionPipelineStatus;
}

export type PayloadPipelineCompleted = WebhookPayload<'pipeline.completed', EventPipelineCompleted>;

export interface EventTaskRunning {
	node: ActionNodeRunTask['params'];
	target: string;
}

export type PayloadTaskRunning = WebhookPayload<'task.running', EventTaskRunning>;

export interface EventTaskRan {
	error: string | null;
	node: ActionNodeRunTask['params'];
	target: string;
}

export type PayloadTaskRan = WebhookPayload<'task.ran', EventTaskRan>;

export interface EventToolInstalling {
	runtime: Runtime;
}

export type PayloadToolInstalling = WebhookPayload<'tool.installing', EventToolInstalling>;

export interface EventToolInstalled {
	error: string | null;
	runtime: Runtime;
}

export type PayloadToolInstalled = WebhookPayload<'tool.installed', EventToolInstalled>;

export interface EventToolchainInstalling {
	spec: ToolchainSpec;
}

export type PayloadToolchainInstalling = WebhookPayload<
	'toolchain.installing',
	EventToolchainInstalling
>;

export interface EventToolchainInstalled {
	error: string | null;
	spec: ToolchainSpec;
}

export type PayloadToolchainInstalled = WebhookPayload<
	'toolchain.installed',
	EventToolchainInstalled
>;

export type PayloadWorkspaceSyncing = WebhookPayload<'workspace.syncing', {}>;

export interface EventWorkspaceSynced {
	error: string | null;
}

export type PayloadWorkspaceSynced = WebhookPayload<'workspace.synced', EventWorkspaceSynced>;

// DEPRECATED

/** @deprecated */
export interface EventActionFinished {
	action: Action;
	error: string | null;
	node: ActionNode;
}

/** @deprecated */
export type PayloadActionFinished = WebhookPayload<'action.finished', EventActionFinished>;

/** @deprecated */
export interface EventPipelineAborted {
	error: string;
}

/** @deprecated */
export type PayloadPipelineAborted = WebhookPayload<'pipeline.aborted', EventPipelineAborted>;

/** @deprecated */
export interface EventPipelineFinished {
	baselineDuration: Duration;
	cachedCount: number;
	context: ActionContext;
	duration: Duration;
	estimatedSavings: Duration | null;
	failedCount: number;
	passedCount: number;
}

/** @deprecated */
export type PayloadPipelineFinished = WebhookPayload<'pipeline.finished', EventPipelineFinished>;

/** @deprecated */
export interface EventTargetRunning {
	target: string;
}

/** @deprecated */
export type PayloadTargetRunning = WebhookPayload<'target.running', EventTargetRunning>;

/** @deprecated */
export interface EventTargetRan {
	error: string | null;
	target: string;
}

/** @deprecated */
export type PayloadTargetRan = WebhookPayload<'target.ran', EventTargetRan>;
