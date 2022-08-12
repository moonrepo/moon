console.log('stdout');
console.log('stderr');

if (process.argv.includes('--fail')) {
	process.exitCode = 1;
}
