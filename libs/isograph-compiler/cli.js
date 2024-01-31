#!/usr/bin/env node
'use strict';

var bin = require('.');
var spawn = require('child_process').spawn;

var input = process.argv.slice(2);

if (bin !== null) {
  spawn(bin, input, { stdio: 'inherit' }).on('exit', process.exit);
} else {
  throw new Error(
    `Platform "${process.platform} (${process.arch})" not supported.`,
  );
}
