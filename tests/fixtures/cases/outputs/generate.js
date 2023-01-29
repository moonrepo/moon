const fs = require('fs');
const path = require('path');

const type = process.argv[2];

function createFile(file, content) {
	const filePath = path.join(__dirname, file);

	fs.mkdirSync(path.dirname(filePath), { recursive: true });
	fs.writeFileSync(filePath, content ?? String(Date.now()), 'utf8');
}

switch (type) {
	case 'single-file':
		createFile('lib/one.js');
		break;
	case 'single-folder':
	case 'multiple-files':
		createFile('lib/one.js');
		createFile('lib/two.js');
		break;
	case 'multiple-folders':
	case 'both':
		createFile('lib/one.js');
		createFile('esm/two.js');
		break;
	case 'multiple-types':
		createFile('build/one.js');
		createFile('build/two.js');
		createFile('build/styles.css');
		createFile('build/image.png');
		break;
	case 'custom':
		createFile(process.argv[3], 'fixed content');
		break;
	case 'none':
		console.log('No outputs!');
		break;
}
