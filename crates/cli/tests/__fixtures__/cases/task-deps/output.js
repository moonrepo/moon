let string = '';

Object.entries(process.env).forEach(([key, value]) => {
	if (key.startsWith('TEST_')) {
		string += `${key}=${value} `;
	}
});

string += process.argv.slice(2).join(' ');

console.log(string);
