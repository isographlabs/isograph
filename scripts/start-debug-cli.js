'use strict';

var path = require('path');
var spawn = require('child_process').spawn;

var input = process.argv.slice(2);

spawn(path.join(__dirname, '../target/debug/isograph_cli'), input, {
  stdio: 'inherit',
}).on('exit', process.exit);
