import path from 'path';

export const ROOT = process.cwd();
export const { BINARY = process.platform === 'win32' ? 'moon.exe' : 'moon', TARGET = '' } =
	process.env;

const targetToPackage = {
	'aarch64-apple-darwin': 'core-macos-arm64',
	'aarch64-unknown-linux-gnu': 'core-linux-x64-gnu',
	'aarch64-unknown-linux-musl': 'core-linux-x64-musl',
	'x86_64-apple-darwin': 'core-macos-x64',
	'x86_64-pc-windows-msvc': 'core-windows-x64-msvc',
	'x86_64-unknown-linux-gnu': 'core-linux-x64-gnu',
	'x86_64-unknown-linux-musl': 'core-linux-x64-musl',
};

export function getPackageFromTarget(target = TARGET) {
	if (targetToPackage[target]) {
		return targetToPackage[target];
	}

	throw new Error(`Unsupported target "${target}".`);
}

export function getPath(...parts) {
	return path.join(ROOT, ...parts);
}
