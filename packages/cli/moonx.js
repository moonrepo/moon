#!/usr/bin/env node

const cp = require('child_process');
const { findMoonxExe } = require('./utils');

const result = cp.spawnSync(findMoonxExe(), process.argv.slice(2), {
	shell: false,
	stdio: 'inherit',
});

if (result.error) {
	throw result.error;
}

process.exitCode = result.status;
