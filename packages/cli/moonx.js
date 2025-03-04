#!/usr/bin/env node

const cp = require('child_process');
const path = require('path');

const result = cp.spawnSync(
	path.join(__dirname, process.platform === 'win32' ? 'moon.exe' : 'moon'),
	['run', ...process.argv.slice(2)],
	{
		shell: false,
		stdio: 'inherit',
	},
);

if (result.error) {
	throw result.error;
}

process.exitCode = result.status;
