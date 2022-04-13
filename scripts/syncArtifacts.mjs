import fs from 'fs';
import path from 'path';
import { getPackageFromTarget, getPath } from './helpers.mjs';

async function syncArtifacts() {
	const targetDirs = await fs.promises.readdir(getPath('artifacts'));

	await Promise.all(
		targetDirs.map(async (targetDir) => {
			const artifacts = await fs.promises.readdir(getPath('artifacts', targetDir));

			await Promise.all(
				artifacts.map(async (artifact) => {
					const artifactPath = getPath('artifacts', targetDir, artifact);
					const target = targetDir.replace('binary-', '');

					// Copy the artifact binary into the target core package
					const binaryPath = getPath('packages', getPackageFromTarget(target), artifact);

					await fs.promises.copyFile(artifactPath, binaryPath);
					await fs.promises.chmod(binaryPath, 0o755);

					// Copy the artifact binary into the release folder so it can be used as an asset
					const releasePath = getPath(
						'artifacts/release',
						artifact.replace('moon', `moon-${target}`),
					);

					await fs.promises.mkdir(path.dirname(releasePath), { recursive: true });
					await fs.promises.copyFile(artifactPath, releasePath);
				}),
			);
		}),
	);
}

syncArtifacts().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
