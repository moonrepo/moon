import fs from 'fs/promises';
import chalk from 'chalk';
import { execa } from 'execa';

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

async function run() {
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

	// Sync the cli version to the cli Cargo.toml
	if (diff.some((file) => file.includes('@moonrepo/cli'))) {
		await syncCargoVersion(prevVersions['@moonrepo/cli'], nextVersions['@moonrepo/cli']);
	}

	// Create git commit and tags
	await createCommit(diff);
	await createTags(diff);

	console.log(chalk.green('Created commit and tags!'));
}

await run();
