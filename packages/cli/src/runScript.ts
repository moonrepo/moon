import { spawnSync } from 'child_process';
import fs from 'fs';
import path from 'path';

const workspaceRoot = process.env.MOON_WORKSPACE_ROOT ?? process.cwd();
const projectRoot = process.env.MOON_PROJECT_ROOT ?? process.cwd();
const scriptName = process.argv[2];

if (!scriptName) {
	throw new Error('A script name is required as the 1st positional argument.');
}

// We need to determine which package manager to run with
let packageManager = 'npm';

if (fs.existsSync(path.join(workspaceRoot, 'yarn.lock'))) {
	packageManager = 'yarn';
} else if (fs.existsSync(path.join(workspaceRoot, 'pnpm-lock.yaml'))) {
	packageManager = 'pnpm';
}

// Execute the script as a child process
spawnSync(packageManager, ['run', scriptName], {
	cwd: projectRoot,
	// eslint-disable-next-line no-magic-numbers
	maxBuffer: 1000 * 1000 * 100, // execa
	stdio: 'inherit',
});
