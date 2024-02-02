Object.entries(Deno.env.toObject())
	.sort((a, d) => a[0].localeCompare(d[0]))
	.forEach(([key, value]) => {
		if (
			key.startsWith('MOON_') &&
			!key.startsWith('MOON_TEST') &&
			key !== 'MOON_VERSION' &&
			key !== 'MOON_APP_LOG'
		) {
			console.log(`${key}=${value.replace(/\\/g, '/')}`);
		}
	});
