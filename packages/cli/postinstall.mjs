// Based on the great parcel-css
// https://github.com/parcel-bundler/parcel-css/blob/master/cli/postinstall.js

import fs from 'fs';
import path from 'path';

const parts = [process.platform, process.arch];

if (process.platform === 'linux') {
	const { MUSL, family } = require('detect-libc');

	if ((await family()) === MUSL) {
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
const target = parts.join('-');
let pkgPath;

try {
	pkgPath = path.dirname(require.resolve(`@moonrepo/core-${target}/package.json`));

	if (!fs.existsSync(path.join(pkgPath, binary))) {
		throw new Error('Target not built.');
	}
} catch {
	pkgPath = path.join(__dirname, '../../../target/release');

	if (!fs.existsSync(path.join(pkgPath, binary))) {
		pkgPath = path.join(__dirname, '../../../target/debug');
	}
}

try {
	await fs.promises.link(path.join(pkgPath, binary), path.join(__dirname, '..', binary));
} catch {
	try {
		await fs.promises.copyFile(path.join(pkgPath, binary), path.join(__dirname, '..', binary));
	} catch {
		throw new Error('Failed to find "moon" binary.');
	}
}
