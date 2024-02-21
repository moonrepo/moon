import parser from 'yargs-parser';
import { json, type PackageStructure } from '@boost/common';
import { type ExecutorsJson, findPackageRoot } from './nx';

export async function execute(argv: string[]) {
	// The executor to run: @nx/webpack:webpack
	const [pkgName, executorName] = (argv.shift() ?? '').split(':');

	if (!pkgName || !executorName) {
		throw new Error('Invalid executor format, expected `@scope/package:executor`.');
	}

	// Options to pass to the executor function
	const args = parser(argv, {
		configuration: {
			'populate--': true,
		},
	});

	// Find the package root and load the package.json
	const pkgRoot = await findPackageRoot(`${pkgName}/package.json`);
	const pkg: PackageStructure = json.load(`${pkgRoot}/package.json`);

	// Find the executors.json file and load it
	const executorsPath = pkgRoot.append(pkg.executors ?? './executors.json');

	if (!executorsPath.exists()) {
		throw new Error(
			`Unable to find executors file \`${executorsPath}\` for package \`${pkgName}\`.`,
		);
	}

	const executorsMap: ExecutorsJson = json.load(executorsPath);
	const executor = executorsMap[executorName];

	if (!executor) {
		throw new Error(`Executor \`${executorName}\` not found in \`${executorsPath}\`.`);
	}
}
