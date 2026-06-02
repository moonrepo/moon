const fs = require('fs');
const path = require('path');

const type = process.argv[2];
const inWorkspace = process.argv.includes('--workspace');

function createFile(file, content) {
	const filePath = path.join(inWorkspace ? process.env.MOON_WORKSPACE_ROOT : __dirname, type, file);

	fs.mkdirSync(path.dirname(filePath), { recursive: true });
	fs.writeFileSync(filePath, content ?? String(Date.now()), 'utf8');
}

switch (type) {
	case 'single-file':
		createFile('one.js');
		break;
	case 'single-folder':
	case 'multiple-files':
		createFile('one.js');
		createFile('two.js');
		break;
	case 'multiple-folders':
	case 'both':
		createFile('a/one.js');
		createFile('b/two.js');
		break;
	case 'multiple-types':
		createFile('one.js');
		createFile('two.js');
		createFile('styles.css');
		createFile('image.png');
		break;
	case 'custom':
		createFile(process.argv[3], 'fixed content');
		break;
	case 'none':
		console.log('No outputs!');
		break;
}
