/* eslint-disable @typescript-eslint/prefer-nullish-coalescing */

import { execa } from 'execa';

export async function getBaseHash() {
	const base = process.env.GITHUB_BASE_REF || 'master';
	const bases = [base, `origin/${base}`, `upstream/${base}`];

	for (const ref of bases) {
		try {
			// eslint-disable-next-line no-await-in-loop
			const result = await execa('git', ['merge-base', ref, 'HEAD'], {
				stdio: 'pipe',
			});

			return result.stdout;
		} catch {
			// Ignore
		}
	}

	return base;
}

export async function getChangedFiles() {
	const base = await getBaseHash();

	return (
		await execa('git', ['diff', '--name-only', base], {
			stdio: 'pipe',
		})
	).stdout.split('\n');
}
