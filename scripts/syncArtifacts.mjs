import fs from 'fs';
import { BINARY, getPackageFromTarget, getPath } from './helpers.mjs';

async function syncArtifacts() {
	const targetDirs = await fs.promises.readdir(getPath('artifacts'));

	await Promise.all(
		targetDirs.map(async (targetDir) => {
			const artifactPath = getPath('artifacts', targetDir, BINARY);
			const binaryPath = getPath(
				'packages',
				getPackageFromTarget(targetDir.replace('binary-')),
				BINARY,
			);

			// Copy the artifact binary into the target core package
			await fs.promises.copyFile(artifactPath, binaryPath);
			await fs.promises.chmod(binaryPath, 0o755);
		}),
	);
}

syncArtifacts().catch((error) => {
	console.error(error);
	process.exit(1);
});
