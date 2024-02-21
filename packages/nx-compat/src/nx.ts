/* eslint-disable no-magic-numbers */
/* eslint-disable promise/prefer-await-to-callbacks */

import fs from 'node:fs';
import { CachedInputFileSystem, ResolverFactory } from 'enhanced-resolve';
import { Path } from '@boost/common';

declare module '@boost/common' {
	interface PackageStructure {
		executors?: string;
	}
}

export interface ExecutorEntry {
	implementation: string;
	schema: string;
}

export type ExecutorsJson = Record<string, ExecutorEntry>;

const packageResolver = ResolverFactory.createResolver({
	enforceExtension: true,
	extensions: ['.json'],
	fileSystem: new CachedInputFileSystem(fs, 4000),
});

export async function findPackageRoot(pkgName: string): Promise<Path> {
	return new Promise((resolve, reject) => {
		packageResolver.resolve({}, process.cwd(), pkgName, {}, (error, result) => {
			if (error) {
				reject(error);
			} else if (result) {
				resolve(new Path(result.replace('package.json', '')));
			} else {
				reject(new Error(`Unable to resolve location of \`${pkgName}\`.`));
			}
		});
	});
}

export function findExecutorImplPath(
	pkgRoot: string,
	executorEntry: ExecutorEntry,
): [Path, string] {
	const [impl, exportName = 'default'] = executorEntry.implementation.split('#');
	const exts = ['.js', '.mjs', '.cjs'];
	const lookups = [impl, impl.replace('/src/', '/lib/'), impl.replace('/src/', '/build/')];

	for (const lookup of lookups) {
		for (const ext of exts) {
			const lookupPath = new Path(pkgRoot, lookup + ext);

			if (lookupPath.exists()) {
				return [lookupPath, exportName];
			}
		}
	}

	throw new Error('TODO');
}
