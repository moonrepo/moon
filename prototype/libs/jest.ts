import { Task, TaskUserOptions } from '../types';

export interface TestOptions {
	cache?: boolean;
	extensions?: string[];
}

export function test(options: TaskUserOptions, { cache = true, extensions }: TestOptions): Task {
	const args = ['--testMatch', '@in(1)'];

	if (cache) {
		args.push('--cache', '--cacheDirectory', '@cache(jest/)');
	}

	return {
		args,
		binary: 'eslint',
		inputs: ['@root(sources)', '@root(tests)', 'jest.config.*', '/jest.config.*'],
		options: {
			...options,
			debugOption: '--debug',
			watchOption: '--watch',
		},
		outputs: [],
		type: 'test',
	};
}
