#!/usr/bin/env node
'use strict';

var bin = require('.');
var spawn = require('child_process').spawn;
var chmodSync = require('fs').chmodSync;

var input = process.argv.slice(2);

if (bin != null) {
  try {
    // Force 755 (rwxr-xr-x)
    //
    // If Isograph is installed with Bun, the binary isn't always executable.
    chmodSync(bin, 0o755);
  } catch {}

  spawn(bin, input, { stdio: 'inherit' }).on('exit', process.exit);
} else {
  throw new Error(
    `Platform "${process.platform} (${process.arch})" not supported.`,
  );
}
