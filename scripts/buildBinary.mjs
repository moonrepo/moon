// We cant use npm dependencies as these scripts run before `yarn install`

import { spawn } from 'child_process';
import path from 'path';
import fs from 'fs';

const ROOT = process.cwd();
const BINARY = process.platform === 'win32' ? 'moon.exe' : 'moon';
const { TARGET } = process.env;

if (!TARGET) {
	throw new Error('TARGET required for building.');
}

// Allow arbitrary args to be passed through
const args = process.argv.slice(2);

// Build the binary with the provided target
await new Promise((resolve, reject) => {
	const child = spawn(
		'cargo',
		['build', '--release', '--features', 'cli', '--target', TARGET, ...args],
		{
			stdio: 'inherit',
			cwd: ROOT,
			shell: true,
		},
	);

	child.on('error', reject);
	child.on('close', resolve);
});

// Copy the binary to the package
const targetToPackage = {
	'x86_64-apple-darwin': 'core-darwin-x64',
	'x86_64-pc-windows-msvc': 'core-win32-x64-msvc',
	'x86_64-unknown-linux-gnu': 'core-linux-x64-gnu',
	'x86_64-unknown-linux-musl': 'core-linux-x64-musl',
};

if (targetToPackage[TARGET]) {
	const artifactPath = path.join(ROOT, `artifacts/bindings-${TARGET}`, BINARY);
	const targetPath = path.join(ROOT, 'target', TARGET, 'release', BINARY);
	const binPath = path.join(ROOT, 'packages', targetToPackage[TARGET], 'moon');

	await fs.promises.copyFile(fs.existsSync(artifactPath) ? artifactPath : targetPath, binPath);
	await fs.promises.chmod(binPath, 0o755);
} else {
	throw new Error(`Unsupported target "${TARGET}".`);
}
