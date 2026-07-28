import proc from 'node:process';
import readline from 'node:readline';

const { stdin: input, stdout: output } = proc;
const rl = readline.createInterface({ input, output });

rl.question('Question? ', (answer) => {
	console.log(`Answer: ${answer}`);
	rl.close();
});
