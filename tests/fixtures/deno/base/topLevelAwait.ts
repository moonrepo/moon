console.log('before');

await new Promise((resolve) => {
	setTimeout(() => {
		console.log('awaiting');
		resolve();
	}, 100);
});

console.log('after');
