import fs from 'fs';
import chalk from 'chalk';
import yaml from 'yaml';
import { getChangedFiles } from '../git.mjs';

async function run() {
	const changedFiles = await getChangedFiles();
	const hasRustChanges = changedFiles.some(
		(file) => file.startsWith('crates') && file.endsWith('.rs') && !file.endsWith('_test.rs'),
	);

	// Exit if no changes to Rust code
	if (!hasRustChanges) {
		return;
	}

	// Load each version file and check for the cli/core packages
	const versions = fs.existsSync('.yarn/versions')
		? await fs.promises.readdir('.yarn/versions')
		: [];

	const hasVersionBump = versions.some((version) => {
		const contents = yaml.parse(fs.readFileSync(`.yarn/versions/${version}`, 'utf8'));
		const bump = contents?.releases?.['@moonrepo/cli'];

		return bump === 'major' || bump === 'minor' || bump === 'patch';
	});

	if (!hasVersionBump) {
		process.exitCode = 1;

		console.error(
			`Detected changes to Rust code but no version bump for the CLI package. Run ${chalk.magenta(
				'yarn version:bump:bin <major|minor|patch>',
			)} to bump.`,
		);
	}
}

await run();
