import type {
	Executor,
	ExecutorContext,
	ExecutorsJson,
	ExecutorsJsonEntry,
} from 'nx/src/config/misc-interfaces';
import parser from 'yargs-parser';
import { json, type PackageStructure, Path } from '@boost/common';
import { env, findPackageRoot } from './helpers';
import {
	createNxProjectGraphFromMoonProjectGraph,
	createNxTargetFromSnapshot,
	createNxTaskGraphFromMoonGraphs,
	loadActionGraph,
	loadProjectGraph,
	loadProjectSnapshot,
} from './moon';
import { loadNxJson, loadWorkspaceJson } from './nx';

async function createExecutorContext(executorTarget: string): Promise<ExecutorContext> {
	const root = env('MOON_WORKSPACE_ROOT');
	const [projectName, targetName] = env('MOON_TARGET').split(':');
	const [nxJson, workspaceJson, snapshot, projectGraph, actionGraph] = await Promise.all([
		loadNxJson(root),
		loadWorkspaceJson(root),
		loadProjectSnapshot(),
		loadProjectGraph(root),
		loadActionGraph(root),
	]);

	return {
		cwd: env('MOON_WORKING_DIR'),
		isVerbose: false,
		nxJsonConfiguration: nxJson,
		projectGraph: createNxProjectGraphFromMoonProjectGraph(projectGraph),
		projectName,
		projectsConfigurations: workspaceJson,
		root,
		target: createNxTargetFromSnapshot(snapshot, executorTarget, targetName),
		targetName,
		taskGraph: createNxTaskGraphFromMoonGraphs(actionGraph, projectGraph),
		workspace: {
			...nxJson,
			...workspaceJson,
		},
	};
}

function findExecutorImplPath(
	pkgRoot: Path,
	pkgName: string,
	executorEntry: ExecutorsJsonEntry,
): [Path, string] {
	const entry =
		typeof executorEntry === 'string' ? { implementation: executorEntry } : executorEntry;
	const [implFile, exportName = 'default'] = entry.implementation.split('#');
	const exts = ['', '.js', '.mjs', '.cjs'];
	const lookups = [
		implFile,
		implFile.replace('/src/', '/lib/'),
		implFile.replace('/src/', '/build/'),
	];

	for (const lookup of lookups) {
		for (const ext of exts) {
			const lookupPath = pkgRoot.append(lookup + ext);

			if (lookupPath.exists()) {
				return [lookupPath, exportName];
			}
		}
	}

	throw new Error(
		`Unable to find executors implementation \`${implFile}\` for package \`${pkgName}\`.`,
	);
}

export async function execute(argv: string[]) {
	// The executor to run: @nx/webpack:webpack
	const executorTarget = argv.shift() ?? '';
	const [pkgName, executorName] = executorTarget.split(':');

	if (!pkgName || !executorName) {
		throw new Error('Invalid executor format, expected `@scope/package:executor`.');
	}

	// Find the package root and load the package.json
	const pkgRoot = await findPackageRoot(`${pkgName}/package.json`);
	const pkg: PackageStructure = json.load(`${pkgRoot}/package.json`);

	// Find the executors.json file and load it
	const executorsFile = pkg.executors ?? './executors.json';
	const executorsPath = pkgRoot.append(executorsFile);

	if (!executorsPath.exists()) {
		throw new Error(
			`Unable to find executors file \`${executorsFile}\` for package \`${pkgName}\`.`,
		);
	}

	const executorsMap: ExecutorsJson = json.load(executorsPath);
	const executor = executorsMap.executors?.[executorName];

	if (!executor) {
		throw new Error(`Executor \`${executorName}\` not found in \`${executorsPath}\`.`);
	}

	// Find the executor implementation file and import it
	const [implPath, exportName] = findExecutorImplPath(pkgRoot, pkgName, executor);
	const impls = (await import(implPath.path())) as Record<string, Executor>;
	const func = impls[exportName];

	if (!func) {
		throw new Error(`Executor implementation \`${exportName}\` not found in \`${implPath}\`.`);
	}

	// Options to pass to the executor function
	const { $0, _, ...options } = parser(argv, {
		configuration: {
			'populate--': true,
		},
	});

	// Execute the executor function
	const result = await func(options, await createExecutorContext(executorTarget));

	if ('success' in result && !result.success) {
		process.exitCode = 1;
	}
}
