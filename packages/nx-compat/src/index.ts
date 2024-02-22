import { execute } from './execute';

const [, command, ...argv] = process.argv;

if (command === 'execute') {
	void execute(argv);
} else {
	throw new Error(`Unknown command \`${command}\`.`);
}
