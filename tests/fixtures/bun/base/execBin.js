const { spawnSync } = require('child_process');

spawnSync('node', ['--version'], { stdio: 'inherit' });
