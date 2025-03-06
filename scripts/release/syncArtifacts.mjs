import fs from 'fs/promises';
import { getPackageFromTarget, getPath } from '../helpers.mjs';

async function syncArtifacts() {
	const dirs = await fs.readdir(getPath('artifacts'));

	await fs.mkdir(getPath('artifacts/release'), { recursive: true });

	await Promise.all(
		dirs.map(async (dir) => {
			const artifacts = await fs.readdir(getPath('artifacts', dir));

			await Promise.all(
				artifacts.map(async (artifact) => {
					const artifactPath = getPath('artifacts', dir, artifact);

					// Copy the artifact binary into the target core package
					const target = dir.replace('binary-', '');
					const binaryPath = getPath('packages', getPackageFromTarget(target), artifact);

					await fs.copyFile(artifactPath, binaryPath);
					await fs.chmod(binaryPath, 0o755);

					// Copy the artifact binary into the release folder so it can be used as an asset
					const releasePath = getPath(
						'artifacts/release',
						artifact.replace('moon', `moon-${target}`),
					);

					await fs.copyFile(artifactPath, releasePath);
					await fs.chmod(releasePath, 0o755);
				}),
			);
		}),
	);
}

await syncArtifacts();
