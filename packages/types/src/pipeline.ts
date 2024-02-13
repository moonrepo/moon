import type { Duration, Runtime } from './common';

export type ActionStatus =
	| 'cached-from-remote'
	| 'cached'
	| 'failed-and-abort'
	| 'failed'
	| 'invalid'
	| 'passed'
	| 'running'
	| 'skipped';

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

export interface Action {
	allowFailure: boolean;
	attempts: Attempt[] | null;
	createdAt: string;
	duration: Duration | null;
	error: string | null;
	finishedAt: string | null;
	flaky: boolean;
	label: string | null;
	startedAt: string | null;
	status: ActionStatus;
}

export interface TargetState {
	state: 'completed' | 'failed' | 'passthrough' | 'skipped';
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
	workspaceRoot: string;
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
	// Deprecated
	estimatedSavings?: Duration | null;
	projectedDuration?: Duration;
}

// NODES

export type ActionNode =
	| ActionNodeInstallDeps
	| ActionNodeInstallProjectDeps
	| ActionNodeRunTask
	| ActionNodeSetupTool
	| ActionNodeSyncProject
	| ActionNodeSyncWorkspace;

export interface ActionNodeInstallDeps {
	action: 'InstallDeps';
	params: {
		runtime: Runtime;
	};
}

export interface ActionNodeInstallProjectDeps {
	action: 'InstallProjectDeps';
	params: {
		runtime: Runtime;
		project: string;
	};
}

export interface ActionNodeRunTask {
	action: 'RunTask';
	params: {
		interactive: boolean;
		persistent: boolean;
		runtime: Runtime;
		target: string;
	};
}

export interface ActionNodeSetupTool {
	action: 'SetupTool';
	params: {
		runtime: Runtime;
	};
}

export interface ActionNodeSyncProject {
	action: 'SyncProject';
	params: {
		runtime: Runtime;
		project: string;
	};
}

export interface ActionNodeSyncWorkspace {
	action: 'SyncWorkspace';
	params: {};
}
