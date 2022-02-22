import fs from 'fs';
import { getPackageFromTarget, getPath } from './helpers.mjs';

async function syncArtifacts() {
	const targetDirs = await fs.promises.readdir(getPath('artifacts'));

	console.log(targetDirs);

	await Promise.all(
		targetDirs.map(async (targetDir) => {
			const artifacts = await fs.promises.readdir(getPath('artifacts', targetDir));

			console.log(targetDir, artifacts);

			await Promise.all(
				artifacts.map(async (artifact) => {
					const artifactPath = getPath('artifacts', targetDir, artifact);
					const binaryPath = getPath(
						'packages',
						getPackageFromTarget(targetDir.replace('binary-')),
						artifact,
					);

					// Copy the artifact binary into the target core package
					await fs.promises.copyFile(artifactPath, binaryPath);
					await fs.promises.chmod(binaryPath, 0o755);
				}),
			);
		}),
	);
}

syncArtifacts().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
