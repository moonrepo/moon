// Based on the great parcel-css
// https://github.com/parcel-bundler/parcel-css/blob/master/cli/postinstall.js

const fs = require('fs');
const path = require('path');

const platform =
	process.platform === 'win32'
		? 'windows'
		: process.platform === 'darwin'
		? 'macos'
		: process.platform;
const parts = [platform, process.arch];

if (process.platform === 'linux') {
	const { familySync } = require('detect-libc');

	if (familySync() === 'musl') {
		parts.push('musl');
	} else if (process.arch === 'arm') {
		parts.push('gnueabihf');
	} else {
		parts.push('gnu');
	}
} else if (process.platform === 'win32') {
	parts.push('msvc');
}

const binary = process.platform === 'win32' ? 'moon.exe' : 'moon';
const triple = parts.join('-');

const pkgPath = path.dirname(require.resolve(`@moonrepo/core-${triple}/package.json`));
const binPath = path.join(pkgPath, binary);

try {
	if (fs.existsSync(binPath)) {
		try {
			fs.linkSync(binPath, path.join(__dirname, binary));
		} catch {
			fs.copyFileSync(binPath, path.join(__dirname, binary));
		}
	} else {
		throw new Error();
	}
} catch {
	console.error('Failed to find "moon" binary.');
	// process.exit(1);
}
