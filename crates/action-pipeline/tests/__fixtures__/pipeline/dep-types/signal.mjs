import fs from 'node:fs';
import path from 'node:path';

const [action, name, arg] = process.argv.slice(2);

function sleep(ms) {
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function signal(signalName) {
	fs.writeFileSync(path.join(import.meta.dirname, signalName), 'signaled');
}

function waitFor(signalName) {
	const file = path.join(import.meta.dirname, signalName);
	const timeout = Date.now() + 10_000;

	while (!fs.existsSync(file)) {
		if (Date.now() > timeout) {
			console.error(`Timed out waiting for "${signalName}" signal!`);
			process.exit(1);
		}

		sleep(50);
	}
}

switch (action) {
	// Signal that this task has ran (after an optional delay)
	case 'write': {
		if (arg) {
			sleep(Number(arg));
		}

		signal(name);
		break;
	}

	// Fail immediately if another task has not signaled yet
	case 'check': {
		if (!fs.existsSync(path.join(import.meta.dirname, name))) {
			console.error(`Expected "${name}" signal to exist!`);
			process.exit(1);
		}

		break;
	}

	// Signal that this task has ran, and then fail
	case 'fail': {
		signal(name);
		process.exit(1);
	}

	// Block until another task has signaled
	case 'wait': {
		waitFor(name);
		break;
	}

	// Signal that this task has started, and then block until another
	// task has signaled, so that both must be running at the same time
	case 'serve': {
		signal(name);
		waitFor(arg);
		break;
	}

	// Block until another task has started, and then signal it
	case 'handshake': {
		waitFor(name);
		signal(arg);
		break;
	}
}
