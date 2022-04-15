import fs from 'fs/promises';
import { exec } from '../helpers.mjs';

async function getPackageVersions() {
	const files = await fs.readdir('./packages');
	const versions = {};

	await Promise.all(
		files.map(async (file) => {
			const pkg = JSON.parse(await fs.readFile(`./packages/${file}/package.json`, 'utf8'));

			versions[pkg.name] = pkg.version;
		}),
	);

	return versions;
}

async function run() {
	// Gather the versions before we apply the new ones
	const prevVersions = await getPackageVersions();

	// Apply them via yarn
	await exec('yarn', ['version', 'apply', '--all']);

	// Now gather the versions again so we can diff
	const nextVersions = await getPackageVersions();

	console.log(prevVersions, nextVersions);
}

run().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
