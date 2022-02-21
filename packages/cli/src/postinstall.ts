// Based on the great parcel-css
// https://github.com/parcel-bundler/parcel-css/blob/master/cli/postinstall.js

import fs from 'fs';
import path from 'path';

const parts = [process.platform, process.arch];

if (process.platform === 'linux') {
	const { MUSL, familySync } = require('detect-libc');

	if (familySync() === MUSL) {
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
let pkgPath: string;

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
	fs.linkSync(path.join(pkgPath, binary), path.join(__dirname, '..', binary));
} catch {
	try {
		fs.copyFileSync(path.join(pkgPath, binary), path.join(__dirname, '..', binary));
	} catch {
		throw new Error('Failed to find moon binary.');
	}
}
