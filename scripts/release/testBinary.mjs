import { BINARY, exec, getPackageFromTarget, getPath } from '../helpers.mjs';

// We cant test the binary through yarn: https://github.com/yarnpkg/berry/issues/4146
// So we must execute it directly as a child process.
async function testBinary() {
	const binaryPath = getPath('node_modules', '@moonrepo', getPackageFromTarget(), BINARY);

	// Ensure its "linked" in the package
	await exec(binaryPath, ['--help']);
}

testBinary().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
