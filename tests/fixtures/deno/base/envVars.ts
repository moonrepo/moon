['MOON_FOO', 'MOON_BAR', 'MOON_BAZ'].forEach((key) => {
	if (Deno.env.get(key)) {
		console.log(`${key}=${Deno.env.get(key).replace(/\\/g, '/')}`);
	}
});
