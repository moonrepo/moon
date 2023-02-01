const { spawnSync } = require('child_process');

// Run itself to trigger recursion!
const exe = process.env.MOON_EXECUTED_WITH;

if (exe) {
	spawnSync(exe, ['run', 'postinstallRecursion:noop'], {
		stdio: 'inherit',
	});
}
