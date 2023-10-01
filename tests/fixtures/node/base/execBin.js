const { spawn } = require('child_process');

spawn('node', ['--version'], { stdio: 'inherit' });
