import type { Duration, Id, ToolchainSpec } from './common';

export type ActionPipelineStatus =
	| 'aborted'
	| 'completed'
	| 'interrupted'
	| 'pending'
	| 'terminated';

export type ActionStatus =
	| 'aborted'
	| 'cached-from-remote'
	| 'cached'
	| 'failed'
	| 'invalid'
	| 'passed'
	| 'running'
	| 'skipped'
	| 'timed-out';

// OPERATIONS

export interface OperationBaseFileChange {
	changedFiles?: string[];
}

export interface OperationBaseProcessOutput {
	command?: string | null;
	exitCode?: number | null;
	stderr?: string | null;
	stdout?: string | null;
}

export interface OperationMetaArchiveCreation {
	type: 'archive-creation';
}

export interface OperationMetaHashGeneration {
	type: 'hash-generation';
	hash: string;
}

export interface OperationMetaMutexAcquisition {
	type: 'mutex-acquisition';
}

export interface OperationMetaNoOperation {
	type: 'no-operation';
}

export interface OperationMetaOutputHydration extends OperationBaseProcessOutput {
	type: 'output-hydration';
}

export interface OperationMetaProcessExecution extends OperationBaseProcessOutput {
	type: 'process-execution';
}

export interface OperationMetaSetupOperation extends OperationBaseFileChange {
	type: 'setup-operation';
}

export interface OperationMetaSyncOperation extends OperationBaseFileChange {
	type: 'sync-operation';
}

export interface OperationMetaTaskExecution extends OperationBaseProcessOutput {
	type: 'task-execution';
}

export type OperationMeta =
	| OperationMetaArchiveCreation
	| OperationMetaHashGeneration
	| OperationMetaMutexAcquisition
	| OperationMetaNoOperation
	| OperationMetaOutputHydration
	| OperationMetaProcessExecution
	| OperationMetaSetupOperation
	| OperationMetaSyncOperation
	| OperationMetaTaskExecution;

export interface Operation {
	duration?: Duration | null;
	finishedAt?: string | null;
	id?: Id | null;
	meta: OperationMeta;
	operations?: Operation[];
	plugin?: Id | null;
	startedAt: string;
	status: ActionStatus;
}

// ACTIONS

export interface Action {
	allowFailure: boolean;
	createdAt: string;
	duration?: Duration | null;
	error?: string | null;
	finishedAt?: string | null;
	flaky: boolean;
	label: string;
	node: ActionNode;
	nodeIndex: number;
	operations: Operation[];
	startedAt?: string | null;
	status: ActionStatus;
}

export interface TargetState {
	state: 'failed' | 'passed' | 'passthrough' | 'skipped';
	hash?: string;
}

export interface AffectedProjectState {
	files?: string[];
	tasks?: string[];
	upstream?: Id[];
	downstream?: Id[];
	other: boolean;
}

export interface AffectedTaskState {
	env?: string[];
	files?: string[];
	projects?: string[];
	upstream?: Id[];
	downstream?: Id[];
	other: boolean;
}

export interface Affected {
	projects: Record<string, AffectedProjectState>;
	tasks: Record<string, AffectedTaskState>;
	shouldCheck: boolean;
}

export interface ActionContext {
	affected?: Affected | null;
	changedFiles: string[];
	initialTargets: string[];
	passthroughArgs: string[];
	primaryTargets: string[];
	profile: 'cpu' | 'heap' | null;
	targetStates: Record<string, TargetState>;
}

export interface RunReport {
	actions: Action[];
	context: ActionContext;
	duration: Duration;
	comparisonEstimate: {
		duration: Duration;
		gain: Duration | null;
		loss: Duration | null;
		percent: number;
		tasks: Record<
			string,
			{
				count: number;
				total: Duration;
			}
		>;
	};
}

// NODES

export type ActionNode =
	| ActionNodeInstallDependencies
	| ActionNodeRunTask
	| ActionNodeSetupEnvironment
	| ActionNodeSetupProto
	| ActionNodeSetupToolchain
	| ActionNodeSyncProject
	| ActionNodeSyncWorkspace;

export interface ActionNodeInstallDependencies {
	action: 'install-dependencies';
	params: {
		members?: string[] | null;
		projectId?: Id | null;
		root: string;
		toolchainId: Id;
	};
}

export interface ActionNodeRunTask {
	action: 'run-task';
	params: {
		args: string[];
		env: Record<string, string>;
		interactive: boolean;
		persistent: boolean;
		priority: number;
		target: string;
		id?: number | null;
	};
}

export interface ActionNodeSetupEnvironment {
	action: 'install-environment';
	params: {
		projectId?: Id | null;
		root: string;
		toolchainId: Id;
	};
}

export interface ActionNodeSetupProto {
	action: 'setup-proto';
	params: {
		version: string;
	};
}

export interface ActionNodeSetupToolchain {
	action: 'setup-toolchain';
	params: {
		toolchain: ToolchainSpec;
	};
}

export interface ActionNodeSyncProject {
	action: 'sync-project';
	params: {
		projectId: Id;
	};
}

export interface ActionNodeSyncWorkspace {
	action: 'sync-workspace';
}

// GRAPH

export interface ActionGraphNode {
	id: number;
	label: string;
}

export interface ActionGraphEdge {
	id: number;
	label: string;
	source: number;
	target: number;
}

export interface ActionGraph {
	edges: ActionGraphEdge[];
	nodes: ActionGraphNode[];
}
