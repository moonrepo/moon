let target = process.env.MOON_TARGET;

console.log(`[${target}] Running`);

for (let event of ['SIGHUP', 'SIGINT', 'SIGQUIT', 'SIGTERM', 'SIGBREAK']) {
	process.on(event, (signal, code) => {
		console.log(`[${target}] Received ${signal} (${code})!`);
		process.exit(128 + code);
	});
}

// Cause the process to take a while!
await new Promise((resolve) => {
	setTimeout(resolve, 30000);
});
