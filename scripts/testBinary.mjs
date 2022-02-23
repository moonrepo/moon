import { spawn } from 'child_process';
import { BINARY, getPackageFromTarget, getPath } from './helpers.mjs';

// We cant test the binary through yarn: https://github.com/yarnpkg/berry/issues/4146
// So we must execute it directly as a child process.
async function testBinary() {
	// Ensure its "linked" in the package
	const binaryPath = getPath('node_modules', '@moonrepo', getPackageFromTarget(), BINARY);

	await new Promise((resolve, reject) => {
		const child = spawn(binaryPath, ['--help'], {
			cwd: process.cwd(),
			shell: true,
			stdio: 'inherit',
		});

		child.on('error', reject);
		child.on('close', resolve);
	});
}

testBinary().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
