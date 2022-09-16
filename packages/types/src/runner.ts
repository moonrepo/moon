export interface Duration {
	secs: number;
	nanos: number;
}

export type ActionStatus =
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
	flaky: boolean;
	label: string | null;
	nodeIndex: number;
	status: ActionStatus;
}

export interface ActionContext {
	passthroughArgs: string[];
	primaryTargets: string[];
	profile: 'cpu' | 'heap' | null;
	touchedFiles: string[];
}

export interface RunReport {
	actions: Action[];
	context: ActionContext;
	duration: Duration;
	estimatedSavings: Duration | null;
	projectedDuration: Duration;
}
