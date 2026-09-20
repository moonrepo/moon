import fs from 'node:fs';
import path from 'node:path';

const [action, name, delay] = process.argv.slice(2);
const file = path.join(import.meta.dirname, name);

function sleep(ms) {
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

if (delay) {
	sleep(Number(delay));
}

switch (action) {
	// Signal that this task has ran
	case 'write': {
		fs.writeFileSync(file, 'signaled');
		break;
	}

	// Fail immediately if another task has not signaled yet
	case 'check': {
		if (!fs.existsSync(file)) {
			console.error(`Expected "${name}" signal to exist!`);
			process.exit(1);
		}

		break;
	}

	// Block until another task has signaled
	case 'wait': {
		const timeout = Date.now() + 10_000;

		while (!fs.existsSync(file)) {
			if (Date.now() > timeout) {
				console.error(`Timed out waiting for "${name}" signal!`);
				process.exit(1);
			}

			sleep(50);
		}

		break;
	}
}
