import { execute } from './execute';

const [, command, ...argv] = process.argv;

if (command === 'execute') {
	await execute(argv);
} else {
	throw new Error(`Unknown command \`${command}\`.`);
}
