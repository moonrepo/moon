['MOON_FOO', 'MOON_BAR', 'MOON_BAZ'].forEach((key) => {
	if (process.env[key]) {
		console.log(`${key}=${process.env[key]}`);
	}
});
