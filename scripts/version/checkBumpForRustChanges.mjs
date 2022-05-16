/* eslint-disable @typescript-eslint/prefer-nullish-coalescing */

import fs from 'fs';
import { execa } from 'execa';
import yaml from 'yaml';

console.log(process.env);

async function loadChangedFiles() {
	const mergeBase = (
		await execa('git', ['merge-base', process.env.GITHUB_BASE_REF || 'origin/master', 'HEAD'], {
			stdio: 'pipe',
		})
	).stdout;

	return (
		await execa('git', ['diff', '--name-only', mergeBase], {
			stdio: 'pipe',
		})
	).stdout.split('\n');
}

async function run() {
	const changedFiles = await loadChangedFiles();
	const hasRustChanges = changedFiles.some(
		(file) => file.startsWith('crates') && file.endsWith('.rs'),
	);

	// Exit if no changes to Rust code
	if (!hasRustChanges) {
		return;
	}

	// Load each version file and check for the cli/core packages
	const versions = await fs.promises.readdir('.yarn/versions');

	const hasVersionBump = versions.some((version) => {
		const contents = yaml.parse(fs.readFileSync(`.yarn/versions/${version}`, 'utf8'));
		const bump = contents?.releases?.['@moonrepo/cli'];

		return bump === 'major' || bump === 'minor' || bump === 'patch';
	});

	if (!hasVersionBump) {
		throw new Error(
			`Changes to Rust code detected, but no version bump for the CLI package. Run \`yarn version:bump:bin <major|minor|patch>\` to bump.`,
		);
	}
}

await run();
