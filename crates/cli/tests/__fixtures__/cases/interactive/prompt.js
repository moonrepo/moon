const readline = require('node:readline');
const { stdin: input, stdout: output } = require('node:process');

const rl = readline.createInterface({ input, output });

rl.question('Question? ', (answer) => {
	console.log(`Answer: ${answer}`);
	rl.close();
});
