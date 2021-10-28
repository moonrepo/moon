import { Task, TaskUserOptions } from '../types';

export interface BuildOptions {
	copyFiles?: boolean;
	extensions?: string[];
	outputDir: string;
}

export function build(
	options: TaskUserOptions,
	{ copyFiles = true, extensions, outputDir }: BuildOptions,
): Task {
	const args = ['@in(0)', '--out-dir', '@out(0)'];

	if (copyFiles) {
		args.push('--copy-files');
	}

	if (Array.isArray(extensions)) {
		args.push('--extensions', extensions.join(','));
	}

	return {
		args,
		binary: 'babel',
		inputs: ['@root(sources)', '.babelrc.*', '/babel.config.*'],
		options: {
			...options,
			debugOption: '--verbose',
		},
		outputs: [outputDir],
		type: 'build',
	};
}
