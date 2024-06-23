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

export function env(name: string): string {
	if (process.env[name]) {
		// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
		return process.env[name]!;
	}

	throw new Error(
		`Missing environment variable \`${name}\`. Executor must be ran through moon's pipeline.`,
	);
}

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

const loadCache: Record<string, Record<string, unknown>> = {};

export async function loadAndCacheJson<T>(
	key: string,
	root: string,
	op: () => Promise<T> | T,
): Promise<T> {
	if (!loadCache[key]) {
		loadCache[key] = {};
	}

	if (!loadCache[key][root]) {
		// eslint-disable-next-line require-atomic-updates
		loadCache[key][root] = await op();
	}

	return loadCache[key][root] as T;
}
