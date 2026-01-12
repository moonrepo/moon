import execa from 'execa';
import fs from 'node:fs';
import fsx from 'node:fs/promises';
import path from 'node:path';

function getCorePackageFromTriple(triple) {
	switch (triple) {
		case 'aarch64-apple-darwin':
			return 'core-macos-arm64';

		case 'aarch64-unknown-linux-gnu':
			return 'core-linux-arm64-gnu';

		case 'aarch64-unknown-linux-musl':
			return 'core-linux-arm64-musl';

		case 'x86_64-unknown-linux-gnu':
			return 'core-linux-x64-gnu';

		case 'x86_64-unknown-linux-musl':
			return 'core-linux-x64-musl';

		case 'x86_64-pc-windows-msvc':
			return 'core-windows-x64-msvc';

		default:
			throw new Error(`Unknown target triple: ${triple}`);
	}
}

if (!process.env.PLAN) {
	throw new Error(`Missing dist-manifest PLAN environment variable`);
}

const plan = JSON.parse(process.env.PLAN);

await Promise.all(Object.values(plan.artifacts).map(async (artifact) => {
	if (artifact.kind !== 'executable-zip') {
		return;
	}

	const triple = artifact.target_triples[0];
	const inputFile = path.join('artifacts', artifact.name);
	const outputDir = path.join('artifacts/release', triple);

	if (!fs.existsSync(outputDir)) {
		await fsx.mkdir(outputDir, { recursive: true });
	}

	if (inputFile.endsWith('.zip')) {
		await execa('unzip', ['-q', inputFile, '-d', outputDir]);
	} else {
		await execa('tar', ['-xf', inputFile, '--strip-components', '1', '-C', outputDir]);
	}

	for (const exe of ['moon', 'moonx']) {
		const exeName = exe + (triple.includes('windows') ? '.exe' : '');
		const exePath = path.join(outputDir, exeName);

		if (fs.existsSync(exePath)) {
			await fsx.copyFile(exePath, path.join('packages', getCorePackageFromTriple(triple), exeName));
		} else {
			throw new Error(`Missing expected executable at path: ${exePath}`);
		}
	}
}));

