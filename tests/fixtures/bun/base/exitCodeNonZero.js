console.log('stdout');
console.error('stderr');

process.exitCode = 1;

console.log('This should appear!');
