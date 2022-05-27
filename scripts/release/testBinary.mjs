import { execa } from 'execa';
import { BINARY, getPackageFromTarget, getPath } from '../helpers.mjs';

// We cant test the binary through yarn: https://github.com/yarnpkg/berry/issues/4146
// So we must execute it directly as a child process.
async function testBinary() {
	const binaryPath = getPath('node_modules', '@moonrepo', getPackageFromTarget(), BINARY);

	// Ensure its "linked" in the package
	await execa(binaryPath, ['--help'], { stdio: 'inherit' });
}

await testBinary();
