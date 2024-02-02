console.log('Args:', Deno.args.join(' '));
console.log('Env:', Deno.env.get('MOON_AFFECTED_FILES') || '');
