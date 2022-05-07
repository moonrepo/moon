Object.entries(process.env).forEach(([key, value]) => {
	if (key.startsWith('MOON_') && !key.startsWith('MOON_TEST')) {
		console.log(`${key}=${value.replace(/\\/g, '/')}`);
	}
});
