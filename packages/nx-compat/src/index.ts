import parser from 'yargs-parser';

const [bin, command, ...argv] = process.argv;
const args = parser(argv, {
	configuration: {
		'populate--': true,
	},
});
