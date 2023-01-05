Object.entries(process.env).forEach(([key, value]) => {
	if (key.startsWith('MOON_') && !key.startsWith('MOON_TEST') && key !== 'MOON_VERSION') {
		console.log(`${key}=${value.replace(/\\/g, '/')}`);
	}
});
