const readline = require('readline');

console.log('START');

const rl = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

function ask(question) {
	console.log('ASK');
	rl.question(question, (answer) => {
		console.log('Q');
		rl.write(`The answer received:  ${answer}\n`);

		ask(question);
	});
}

ask('What is your name: ');
