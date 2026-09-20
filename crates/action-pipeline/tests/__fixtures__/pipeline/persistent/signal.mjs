import fs from 'node:fs';
import path from 'node:path';

const [action, name] = process.argv.slice(2);
const file = path.join(import.meta.dirname, name);

function sleep(ms) {
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

if (action === 'write') {
	fs.writeFileSync(file, 'started');
} else {
	const timeout = Date.now() + 10_000;

	while (!fs.existsSync(file)) {
		if (Date.now() > timeout) {
			console.error(`Timed out waiting for "${name}" signal!`);
			process.exit(1);
		}

		sleep(50);
	}
}
