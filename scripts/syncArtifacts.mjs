import fs from 'fs';
import { getPackageFromTarget, getPath } from './helpers.mjs';

const { BINARY = 'moon', TARGET } = process.env;

if (!TARGET) {
	throw new Error('TARGET required for syncing artifacts.');
}

// Copy the artifact binary into the target core package
const artifactPath = getPath(`artifacts/binary-${TARGET}`, BINARY);
const binaryPath = getPath('packages', getPackageFromTarget(TARGET), BINARY);

await fs.promises.copyFile(artifactPath, binaryPath);
await fs.promises.chmod(binaryPath, 0o755);
