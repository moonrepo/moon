console.log('start');

let interrupted = 0;

process.on('SIGINT', () => {
	console.log('interrupted');

	if (interrupted) {
		console.log('Force exiting!');
		process.exit(3);
	} else {
		console.log('Press ctrl+c again to exit');
		interrupted = true;
	}

	setTimeout(() => {
		console.log(`timeout from SIGINT`);
	}, 5000);
});

setTimeout(() => {
	console.log('timeout from main');
}, 5000);
