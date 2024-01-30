'use strict';

const path = require('path');

let binary;
if (process.platform === 'darwin' && process.arch === 'x64') {
  binary = path.join(__dirname, 'artifacts', 'macos-x64', 'isograph_cli');
} else if (process.platform === 'darwin' && process.arch === 'arm64') {
  binary = path.join(__dirname, 'artifacts', 'macos-arm64', 'isograph_cli');
} else if (process.platform === 'linux' && process.arch === 'x64') {
  binary = path.join(__dirname, 'artifacts', 'linux-x64', 'isograph_cli');
} else if (process.platform === 'linux' && process.arch === 'arm64') {
  binary = path.join(__dirname, 'artifacts', 'linux-arm64', 'isograph_cli');
} else if (process.platform === 'win32' && process.arch === 'x64') {
  throw new Error('Platform not supported yet');
  // binary = path.join(__dirname, "artifacts", "win-x64", "isograph_cli.exe");
} else {
  throw new Error('Platform not supported yet');
  // binary = null;
}

module.exports = binary;
