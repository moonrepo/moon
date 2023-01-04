Object.entries(process.env)
	.sort((a, d) => a[0].localeCompare(d[0]))
	.forEach(([key, value]) => {
		if (key.startsWith('MOON_') && !key.startsWith('MOON_TEST')) {
			console.log(`${key}=${value.replace(/\\/g, '/')}`);
		}
	});
