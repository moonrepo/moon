export type FileGroup = 'sources' | 'tests' | 'assets' | 'resources';

export type FileGroupOperator = 'glob' | 'root' | 'dirs' | 'files';

export type FileGroupInput = `${FileGroup}:${FileGroupOperator}`;

export type Target = string & { __brand: 'target' };

export type TaskType = 'build' | 'test' | 'run';

export interface TaskUserOptions {
	dependsOn?: Target[];
	retryCount?: number;
}

export interface TaskOptions extends TaskUserOptions {
	debugOption?: string | string[]; // --debug
	watchOption?: string | string[]; // --watch
}

export interface Task {
	args: string[];
	binary: string;
	inputs: string[];
	metadata?: Record<string, unknown>;
	options: TaskOptions;
	outputs: string[];
	type: TaskType;
}
