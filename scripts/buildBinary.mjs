import { execa } from 'execa';
import path from 'path';
import fs from 'fs';

const ROOT = process.cwd();
const { TARGET } = process.env;

if (!TARGET) {
	throw new Error('TARGET required for building.');
}

// Allow arbitrary args to be passed through
const args = process.argv.slice(2);

// Build the binary with the provided target
await execa('cargo', ['build', '--release', '--features', 'cli', '--target', TARGET, ...args], {
	stdio: 'inherit',
});

// Copy the binary to the package
const targetToPackage = {
	'x86_64-apple-darwin': 'core-darwin-x64',
	'x86_64-pc-windows-msvc': 'core-win32-x64-msvc',
	'x86_64-unknown-linux-gnu': 'core-linux-x64-gnu',
	'x86_64-unknown-linux-musl': 'core-linux-x64-musl',
};

if (targetToPackage[TARGET]) {
	await fs.promises.copyFile(
		path.join(ROOT, 'target', TARGET, 'release/moon'),
		path.join(ROOT, 'packages', targetToPackage[TARGET], 'moon'),
	);
} else {
	throw new Error(`Unsupported target "${TARGET}".`);
}
