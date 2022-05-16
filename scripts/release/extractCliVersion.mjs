// We need to extract the version from the CLI and set it as an
// environment variable so that the softprops/action-gh-release
// action can utilize it.

import fs from 'fs';

const pkg = JSON.parse(fs.readFileSync('packages/cli/package.json', 'utf8'));

// Must match our tag format!
process.env.NPM_TAG_NAME = `${pkg.name}@${pkg.version}`;
