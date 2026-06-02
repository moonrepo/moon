console.log('stdout');
console.error('stderr');

if (process.argv.includes('--fail')) {
	process.exitCode = 1;
}
