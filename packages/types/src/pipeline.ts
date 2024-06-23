import type { Duration, Runtime } from './common';

export type ActionStatus =
	| 'aborted'
	| 'cached-from-remote'
	| 'cached'
	| 'failed-and-abort' // Legacy
	| 'failed'
	| 'invalid'
	| 'passed'
	| 'running'
	| 'skipped'
	| 'timed-out';

/** @deprecated */
export interface Attempt {
	duration: Duration | null;
	exitCode: number | null;
	finishedAt: string | null;
	index: number;
	startedAt: string;
	status: ActionStatus;
	stderr: string | null;
	stdout: string | null;
}

// OPERATIONS

export interface OperationMetaBaseOutput {
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

export interface OperationMetaOutputHydration extends OperationMetaBaseOutput {
	type: 'output-hydration';
}

export interface OperationMetaSyncOperation {
	type: 'sync-operation';
	label: string;
}

export interface OperationMetaTaskExecution extends OperationMetaBaseOutput {
	type: 'task-execution';
}

export type OperationMeta =
	| OperationMetaArchiveCreation
	| OperationMetaHashGeneration
	| OperationMetaMutexAcquisition
	| OperationMetaNoOperation
	| OperationMetaOutputHydration
	| OperationMetaSyncOperation
	| OperationMetaTaskExecution;

export interface Operation {
	duration: Duration | null;
	finishedAt: string | null;
	meta: OperationMeta;
	startedAt: string;
	status: ActionStatus;
}

// ACTIONS

export interface Action {
	allowFailure: boolean;
	/** @deprecated */
	attempts: Attempt[] | null;
	createdAt: string;
	duration: Duration | null;
	error: string | null;
	finishedAt: string | null;
	flaky: boolean;
	label: string;
	node: ActionNode;
	nodeIndex: number;
	operations: Operation[];
	startedAt: string | null;
	status: ActionStatus;
}

export interface TargetState {
	state: 'failed' | 'passed' | 'passthrough' | 'skipped';
	hash?: string;
}

export interface ActionContext {
	affectedOnly: boolean;
	initialTargets: string[];
	passthroughArgs: string[];
	primaryTargets: string[];
	profile: 'cpu' | 'heap' | null;
	targetStates: Record<string, TargetState>;
	touchedFiles: string[];
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
	| ActionNodeInstallProjectDeps
	| ActionNodeInstallWorkspaceDeps
	| ActionNodeRunTask
	| ActionNodeSetupToolchain
	| ActionNodeSyncProject
	| ActionNodeSyncWorkspace;

export interface ActionNodeInstallWorkspaceDeps {
	action: 'install-workspace-deps';
	params: {
		runtime: Runtime;
	};
}

export interface ActionNodeInstallProjectDeps {
	action: 'install-project-deps';
	params: {
		runtime: Runtime;
		project: string;
	};
}

export interface ActionNodeRunTask {
	action: 'run-task';
	params: {
		args: string[];
		env: Record<string, string>;
		interactive: boolean;
		persistent: boolean;
		runtime: Runtime;
		target: string;
		timeout: number | null;
		id: number | null;
	};
}

export interface ActionNodeSetupToolchain {
	action: 'setup-toolchain';
	params: {
		runtime: Runtime;
	};
}

export interface ActionNodeSyncProject {
	action: 'sync-project';
	params: {
		runtime: Runtime;
		project: string;
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
