console.log('stdout');
console.error('stderr');

new Promise((resolve, reject) => {
	reject('Oops');
});
