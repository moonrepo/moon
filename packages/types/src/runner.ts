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
	finishedAt: string | null;
	index: number;
	startedAt: string;
	status: ActionStatus;
}

export interface Action {
	attempts: Attempt[] | null;
	createdAt: string;
	duration: Duration | null;
	error: string | null;
	finishedAt: string | null;
	flaky: boolean;
	label: string | null;
	nodeIndex: number;
	status: ActionStatus;
}

export interface RunnerContext {
	affectedOnly: boolean;
	initialTargets: string[];
	passthroughArgs: string[];
	primaryTargets: string[];
	profile: 'cpu' | 'heap' | null;
	targetHashes: Record<string, string>;
	touchedFiles: string[];
}

export interface RunReport {
	actions: Action[];
	context: RunnerContext;
	duration: Duration;
	estimatedSavings: Duration | null;
	projectedDuration: Duration;
}

// NODES

export type ActionNode =
	| ActionNodeInstallDeps
	| ActionNodeInstallProjectDeps
	| ActionNodeRunTarget
	| ActionNodeSetupTool
	| ActionNodeSyncProject;

export interface ActionNodeInstallDeps {
	action: 'InstallDeps';
	params: Runtime;
}

export interface ActionNodeInstallProjectDeps {
	action: 'InstallProjectDeps';
	params: [Runtime, string];
}

export interface ActionNodeRunTarget {
	action: 'RunTarget';
	params: [Runtime, string];
}

export interface ActionNodeSetupTool {
	action: 'SetupTool';
	params: Runtime;
}

export interface ActionNodeSyncProject {
	action: 'SyncProject';
	params: [Runtime, string];
}
