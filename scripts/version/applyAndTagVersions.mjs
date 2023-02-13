import { existsSync } from 'fs';
import fs from 'fs/promises';
import chalk from 'chalk';
import { execa } from 'execa';
// eslint-disable-next-line import/no-unresolved
import readline from 'readline/promises';

const rl = readline.createInterface({ input: process.stdin, output: process.stdout });

async function getPackageVersions() {
	const files = await fs.readdir('packages');
	const versions = {};

	await Promise.all(
		files.map(async (file) => {
			const pkg = JSON.parse(await fs.readFile(`packages/${file}/package.json`, 'utf8'));

			versions[pkg.name] = pkg.version;
		}),
	);

	return versions;
}

function logDiff(diff) {
	console.log(`Found ${diff.length} packages to release`);

	diff.forEach((row) => {
		console.log(chalk.gray(`  - ${row}`));
	});
}

async function syncCargoVersion(oldVersion, newVersion) {
	console.log('Syncing version to cli/Cargo.toml');

	let toml = await fs.readFile('crates/cli/Cargo.toml', 'utf8');

	toml = toml.replace(`version = "${oldVersion}"`, `version = "${newVersion}"`);

	await fs.writeFile('crates/cli/Cargo.toml', toml, 'utf8');

	await execa('cargo', ['check'], { stdio: 'inherit' });
}

async function releaseChangelog(newVersion) {
	console.log('Releasing version in changelog');

	let changelog = await fs.readFile('packages/cli/CHANGELOG.md', 'utf8');

	changelog = changelog.replace('## Unreleased', `## ${newVersion}`);

	await fs.writeFile('packages/cli/CHANGELOG.md', changelog, 'utf8');
}

async function removeLocalBuilds() {
	console.log('Removing local builds');

	try {
		await Promise.all(
			['linux-x64-gnu', 'linux-x64-musl', 'macos-arm64', 'macos-x64', 'windows-x64-msvc'].map(
				async (target) => {
					const binPath = `packages/core-${target}/moon${target.includes('windows') ? '.exe' : ''}`;

					if (existsSync(binPath)) {
						await fs.unlink(binPath);
					}
				},
			),
		);

		if (existsSync('target/release')) {
			await fs.rm('target/release', { force: true, recursive: true });
		}
		// eslint-disable-next-line @typescript-eslint/no-implicit-any-catch
	} catch (error) {
		console.error(error.message);
	}
}

async function createCommit(versions) {
	console.log('Creating git commit');

	let commit = 'Release';

	versions.forEach((version) => {
		commit += `\n- ${version}`;
	});

	await execa('git', ['add', '--all'], { stdio: 'inherit' });
	await execa('git', ['commit', '-m', commit], { stdio: 'inherit' });
}

async function createTags(versions) {
	console.log('Creating git tags');

	await Promise.all(
		versions.map(async (version) => {
			await execa('git', ['tag', version]);
		}),
	);
}

async function resetGit() {
	await execa('git', ['reset', '--hard']);
}

async function run() {
	// Delete local builds so we dont inadvertently release it
	await removeLocalBuilds();

	// Gather the versions before we apply the new ones
	const prevVersions = await getPackageVersions();

	// Apply them via yarn
	await execa('yarn', ['version', 'apply', '--all'], { stdio: 'inherit' });

	// Now gather the versions again so we can diff
	const nextVersions = await getPackageVersions();

	// Diff the versions and find the new ones
	const diff = [];

	Object.entries(nextVersions).forEach(([name, version]) => {
		if (version !== prevVersions[name]) {
			diff.push(`${name}@${version}`);
		}
	});

	if (diff.length === 0) {
		console.log(chalk.yellow('No packages to release'));
		return;
	}

	logDiff(diff);

	const answer = await rl.question(`Release (Y/n)? `);
	rl.close();

	if (answer.toLocaleLowerCase() === 'n') {
		await resetGit();
		return;
	}

	// Sync the cli version to the cli Cargo.toml
	if (diff.some((file) => file.includes('@moonrepo/cli'))) {
		await syncCargoVersion(prevVersions['@moonrepo/cli'], nextVersions['@moonrepo/cli']);
		await releaseChangelog(nextVersions['@moonrepo/cli']);
	}

	// Create git commit and tags
	await createCommit(diff);
	await createTags(diff);

	console.log(chalk.green('Created commit and tags!'));
}

await run();
