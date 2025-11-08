import glob from 'fast-glob';
import fs from 'node:fs/promises';
import path from 'node:path';

const files = await glob('**/tasks/*.yml', {
	absolute: true,
	ignore: ['**/node_modules/**', '**/dist/**', '**/build/**']
});

async function migrateConfig(filePath) {
	const name = path.basename(filePath, '.yml');

	if (name === "moon" || name === "all" || name.startsWith('global-')) {
		console.log(`Skipped: ${filePath}`);
		return;
	}

	let by = 'inheritedBy:\n';

	if (name.startsWith('tag-')) {
		const tagName = name.slice(4);

		by += `  tags: ['${tagName}']\n`;
	} else {
		const [toolchain, layer] = name.split('-');

		if (toolchain) {
			by += `  toolchains: '${toolchain}'\n`;
		}

		if (layer) {
			by += `  layers: '${layer}'\n`;
		}
	}

	const content = await fs.readFile(filePath, 'utf8');

	fs.writeFile(filePath, `${by}\n${content}`, 'utf8');

	console.log(`Migrated: ${filePath}`);
}

await Promise.all(files.map(migrateConfig));
