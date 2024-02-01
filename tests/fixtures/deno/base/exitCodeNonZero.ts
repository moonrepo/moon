console.log('stdout');
console.error('stderr');

Deno.exit(1);

console.log('This should not appear!');
