/* eslint-disable @typescript-eslint/no-unsafe-return */

// We do *not* support 32-bit at this time

const supportedPlatforms: Record<string, string> = {
	darwin: 'macos',
	linux: 'linux',
	win32: 'windows',
};

const supportedArchs: Record<string, string> = {
	arm64: 'arm64',
	x64: 'x64',
};

const platform = supportedPlatforms[process.platform];
const arch = supportedArchs[process.arch];

if (!platform) {
	throw new Error(`Unsupported platform "${process.platform}".`);
}

if (!arch) {
	throw new Error(`Unsupported architecture "${process.arch}".`);
}

const parts = [platform, arch];

if (platform === 'linux') {
	const { MUSL, familySync } = require('detect-libc');

	if (familySync() === MUSL) {
		parts.push('musl');
	} else {
		parts.push('gnu');
	}
} else if (platform === 'windows') {
	parts.push('msvc');
}

export const BIN = 'moon';
export const TARGET = parts.join('-');

export default (() => {
	try {
		return require(`@moonrepo/core-${TARGET}`);
	} catch {
		return require(`./moon.${TARGET}.node`);
	}
})();
