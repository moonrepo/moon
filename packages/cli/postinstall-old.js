#!/usr/bin/env node

// Based on the great parcel-css
// https://github.com/parcel-bundler/parcel-css/blob/master/cli/postinstall.js

const fs = require('fs');
const path = require('path');

const isMoonLocal =
	fs.existsSync(path.join(__dirname, '../../.moon')) &&
	fs.existsSync(path.join(__dirname, '../../crates'));

const isLinux = process.platform === 'linux';
const isMacos = process.platform === 'darwin';
const isWindows = process.platform === 'win32';

const platform = isWindows ? 'windows' : isMacos ? 'macos' : process.platform;
const arch =
	process.env['npm_config_user_agent'] && process.env['npm_config_user_agent'].match(/^bun.*arm64$/)
		? 'arm64'
		: process.arch; // https://github.com/moonrepo/moon/issues/1103
const parts = [platform, arch];

if (isLinux) {
	const { familySync } = require('detect-libc');

	if (familySync() === 'musl') {
		parts.push('musl');
	} else if (process.arch === 'arm') {
		parts.push('gnueabihf');
	} else {
		parts.push('gnu');
	}
} else if (isWindows) {
	parts.push('msvc');
}

const binary = isWindows ? 'moon.exe' : 'moon';
const triple = parts.join('-');

const pkgPath = path.dirname(require.resolve(`@moonrepo/core-${triple}/package.json`));
const binPath = path.join(pkgPath, binary);

try {
	const linkPath = path.join(__dirname, binary);

	if (fs.existsSync(binPath)) {
		try {
			fs.linkSync(binPath, linkPath);
		} catch {
			fs.copyFileSync(binPath, linkPath);
		}

		fs.chmodSync(linkPath, 0o755);
	} else {
		throw new Error();
	}
} catch {
	console.error(`Failed to find "${binary}" binary.`);

	if (!isMoonLocal) {
		// process.exit(1);
	}
}
