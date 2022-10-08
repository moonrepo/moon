import { Runtime } from './common';
import { Project, Task } from './project';
import { Action, ActionNode, Duration } from './runner';

export interface WebhookPayload<T extends EventType, E> {
	createdAt: string;
	event: E;
	type: T;
}

export type EventType =
	| 'runner.action.finished'
	| 'runner.action.started'
	| 'runner.dependencies.installed'
	| 'runner.dependencies.installing'
	| 'runner.project.synced'
	| 'runner.project.syncing'
	| 'runner.run.aborted'
	| 'runner.run.finished'
	| 'runner.run.started'
	| 'runner.target-output.archived'
	| 'runner.target-output.archiving'
	| 'runner.target-output.cache-check'
	| 'runner.target-output.hydrated'
	| 'runner.target-output.hydrating'
	| 'runner.target.ran'
	| 'runner.target.running'
	| 'runner.tool.installed'
	| 'runner.tool.installing';

export interface EventActionStarted {
	action: Action;
	node: ActionNode;
}

export type PayloadActionStarted = WebhookPayload<'runner.action.started', EventActionStarted>;

export interface EventActionFinished {
	action: Action;
	node: ActionNode;
}

export type PayloadActionFinished = WebhookPayload<'runner.action.finished', EventActionFinished>;

export interface EventDependenciesInstalling {
	projectId: string | null;
	runtime: Runtime;
}

export type PayloadDependenciesInstalling = WebhookPayload<
	'runner.dependencies.installing',
	EventDependenciesInstalling
>;

export interface EventDependenciesInstalled {
	projectId: string | null;
	runtime: Runtime;
}

export type PayloadDependenciesInstalled = WebhookPayload<
	'runner.dependencies.installed',
	EventDependenciesInstalled
>;

export interface EventProjectSyncing {
	projectId: string;
	runtime: Runtime;
}

export type PayloadProjectSyncing = WebhookPayload<'runner.project.syncing', EventProjectSyncing>;

export interface EventProjectSynced {
	projectId: string;
	runtime: Runtime;
}

export type PayloadProjectSynced = WebhookPayload<'runner.project.synced', EventProjectSynced>;

// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface EventRunAborted {}

export type PayloadRunAborted = WebhookPayload<'runner.run.aborted', EventRunAborted>;

export interface EventRunStarted {
	actionsCount: number;
}

export type PayloadRunStarted = WebhookPayload<'runner.run.started', EventRunStarted>;

export interface EventRunFinished {
	duration: Duration;
	cachedCount: number;
	failedCount: number;
	passedCount: number;
}

export type PayloadRunFinished = WebhookPayload<'runner.run.finished', EventRunFinished>;

export interface EventTargetRunning {
	targetId: string;
}

export type PayloadTargetRunning = WebhookPayload<'runner.target.running', EventTargetRunning>;

export interface EventTargetRan {
	targetId: string;
}

export type PayloadTargetRan = WebhookPayload<'runner.target.ran', EventTargetRan>;

export interface EventTargetOutputArchiving {
	hash: string;
	project: Project;
	task: Task;
}

export type PayloadTargetOutputArchiving = WebhookPayload<
	'runner.target-output.archiving',
	EventTargetOutputArchiving
>;

export interface EventTargetOutputArchived {
	archivePath: string;
	hash: string;
	project: Project;
	task: Task;
}

export type PayloadTargetOutputArchived = WebhookPayload<
	'runner.target-output.archived',
	EventTargetOutputArchived
>;

export interface EventTargetOutputHydrating {
	hash: string;
	project: Project;
	task: Task;
}

export type PayloadTargetOutputHydrating = WebhookPayload<
	'runner.target-output.hydrating',
	EventTargetOutputHydrating
>;

export interface EventTargetOutputHydrated {
	archivePath: string;
	hash: string;
	project: Project;
	task: Task;
}

export type PayloadTargetOutputHydrated = WebhookPayload<
	'runner.target-output.hydrated',
	EventTargetOutputHydrated
>;

export interface EventTargetOutputCacheCheck {
	hash: string;
	task: Task;
}

export type PayloadTargetOutputCacheCheck = WebhookPayload<
	'runner.target-output.cache-check',
	EventTargetOutputCacheCheck
>;

export interface EventToolInstalling {
	runtime: Runtime;
}

export type PayloadToolInstalling = WebhookPayload<'runner.tool.installing', EventToolInstalling>;

export interface EventToolInstalled {
	runtime: Runtime;
}

export type PayloadToolInstalled = WebhookPayload<'runner.tool.installed', EventToolInstalled>;
