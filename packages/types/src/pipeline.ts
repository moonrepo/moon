import type { Duration, Runtime, ToolchainSpec } from './common';

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
	| ActionNodeInstallDependencies
	| ActionNodeInstallProjectDeps
	| ActionNodeInstallWorkspaceDeps
	| ActionNodeRunTask
	| ActionNodeSetupEnvironment
	| ActionNodeSetupToolchain
	| ActionNodeSetupToolchainLegacy
	| ActionNodeSyncProject
	| ActionNodeSyncWorkspace;

export interface ActionNodeInstallDependencies {
	action: 'install-dependencies';
	params: {
		projectId: string | null;
		root: string;
		toolchainId: string;
	};
}

export interface ActionNodeInstallWorkspaceDeps {
	action: 'install-workspace-deps';
	params: {
		runtime: Runtime;
		root: string;
	};
}

export interface ActionNodeInstallProjectDeps {
	action: 'install-project-deps';
	params: {
		runtime: Runtime;
		projectId: string;
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
		id: number | null;
	};
}

export interface ActionNodeSetupEnvironment {
	action: 'install-environment';
	params: {
		projectId: string | null;
		root: string;
		toolchainId: string;
	};
}

export interface ActionNodeSetupToolchainLegacy {
	action: 'setup-toolchain-legacy';
	params: {
		runtime: Runtime;
	};
}

export interface ActionNodeSetupToolchain {
	action: 'setup-toolchain';
	params: {
		projectId: string | null;
		spec: ToolchainSpec;
	};
}

export interface ActionNodeSyncProject {
	action: 'sync-project';
	params: {
		projectId: string;
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
