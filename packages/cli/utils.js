const fs = require('fs');
const path = require('path');

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
		// } else if (process.arch === 'arm') {
		// 	parts.push('gnueabihf');
	} else {
		parts.push('gnu');
	}
} else if (isWindows) {
	parts.push('msvc');
}

const triple = parts.join('-');

function findMoonExe() {
	const pkgPath = require.resolve(`@moonrepo/core-${triple}/package.json`);
	const exePath = path.join(path.dirname(pkgPath), isWindows ? 'moon.exe' : 'moon');

	if (fs.existsSync(exePath)) {
		return exePath;
	}

	throw new Error(`moon executable "${exePath}" not found!`);
}

exports.findMoonExe = findMoonExe;
