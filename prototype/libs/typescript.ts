import path from 'path';
import { Task, TaskUserOptions } from '../types';

export interface BuildOptions {
	declarationDir?: string;
	inputDir?: string;
	outputDir: string;
}

export function build(
	options: TaskUserOptions,
	{ declarationDir, inputDir, outputDir }: BuildOptions,
): Task {
	return {
		args: ['--build', '--pretty', inputDir ? path.join('@pid', inputDir) : '@pid'],
		binary: 'tsc',
		inputs: [
			'@glob(sources)',
			'@glob(tests)',
			inputDir ? path.join(inputDir, 'tsconfig.json') : 'tsconfig.json',
			'/tsconfig.json',
			'/tsconfig.*.json',
		],
		metadata: {
			declarationDir: declarationDir || outputDir,
			inputDir,
			outputDir,
		},
		options: {
			...options,
			debugOption: ['--extendedDiagnostics', '--listFiles', '--verbose'],
			watchOption: ['--watch'],
		},
		outputs: [outputDir],
		type: 'build',
	};
}

export function test(options: TaskUserOptions): Task {
	return {
		args: ['--noEmit', '--pretty'],
		binary: 'tsc',
		inputs: ['/tsconfig.json', '/tsconfig.*.json'],
		options: {
			...options,
			debugOption: ['--extendedDiagnostics', '--listFiles'],
			watchOption: ['--watch'],
		},
		outputs: [],
		type: 'test',
	};
}
