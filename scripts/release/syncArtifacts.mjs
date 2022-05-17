import fs from 'fs/promises';
import path from 'path';
import { getPackageFromTarget, getPath } from '../helpers.mjs';

async function syncArtifacts() {
	const targetDirs = await fs.readdir(getPath('artifacts'));

	await Promise.all(
		targetDirs.map(async (targetDir) => {
			const artifacts = await fs.readdir(getPath('artifacts', targetDir));

			await Promise.all(
				artifacts.map(async (artifact) => {
					const artifactPath = getPath('artifacts', targetDir, artifact);
					const target = targetDir.replace('binary-', '');

					// Copy the artifact binary into the target core package
					const binaryPath = getPath('packages', getPackageFromTarget(target), artifact);

					await fs.copyFile(artifactPath, binaryPath);
					await fs.chmod(binaryPath, 0o755);

					// Copy the artifact binary into the release folder so it can be used as an asset
					const releasePath = getPath(
						'artifacts/release',
						artifact.replace('moon', `moon-${target}`),
					);

					await fs.mkdir(path.dirname(releasePath), { recursive: true });
					await fs.copyFile(artifactPath, releasePath);
					await fs.chmod(releasePath, 0o755);
				}),
			);
		}),
	);
}

await syncArtifacts();
