let target = process.env.MOON_TARGET;

console.log(`[${target}] Running`);

for (let event of ['SIGHUP', 'SIGINT', 'SIGQUIT', 'SIGTERM', 'SIGBREAK']) {
	process.on(event, (signal, code) => {
		console.log(`[${target}] Received ${signal} (${code})!`);

		if (target === 'signals:dev-2') {
			process.exit(128 + code);
		} else {
			// Give moon some time to kill it
			setTimeout(() => {
				process.exit(128 + code);
			}, 5000);
		}
	});
}

// Cause the process to take a while!
await new Promise((resolve) => {
	setTimeout(resolve, 30000);
});
