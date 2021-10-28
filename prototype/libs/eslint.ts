import { Task, TaskUserOptions } from '../types';

export interface TestOptions {
	cache?: boolean;
	extensions?: string[];
}

export function test(options: TaskUserOptions, { cache = true, extensions }: TestOptions): Task {
	const args = ['@in(0)', '@in(1)', '--report-unused-disable-directives'];

	if (cache) {
		args.push('--cache', '--cache-location', '@cache(.eslintcache)');
	}

	if (Array.isArray(extensions)) {
		args.push('--ext', extensions.join(','));
	}

	return {
		args,
		binary: 'eslint',
		inputs: ['@root(sources)', '@root(tests)', '.eslintrc.*', '/.eslintrc.*', '/.eslintignore'],
		options: {
			...options,
			debugOption: '--debug',
		},
		outputs: [],
		type: 'test',
	};
}

export interface RunOptions {
	fix?: boolean;
	extensions?: string[];
}

export function run(options: TaskUserOptions, { fix, extensions }: RunOptions): Task {
	const task = test(options, { cache: false, extensions });

	task.type = 'run';

	if (fix) {
		task.args.push('--fix');
	}

	return task;
}
