import { Task, TaskType, TaskUserOptions } from '../types';

function createTask(type: TaskType, options: TaskUserOptions, args: string[]): Task {
	return {
		args: ['@in(0)', '@in(1)', ...args],
		binary: 'prettier',
		inputs: ['@glob(sources)', '@glob(tests)', '/prettier.config.*', '/.prettierignore'],
		options: {
			...options,
			debugOption: ['--loglevel', 'debug'],
		},
		outputs: [],
		type,
	};
}

export function run(options: TaskUserOptions): Task {
	return createTask('run', options, ['--write']);
}

export function test(options: TaskUserOptions): Task {
	return createTask('test', options, ['--check']);
}
