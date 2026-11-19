'use strict';

// Compiles the demo's sources with the Isograph Babel plugin as Node requires
// them.

const path = require('node:path');

const demoDirectory = path.join(__dirname, '..');

require('@babel/register')({
  babelrc: false,
  configFile: false,
  cache: false,
  extensions: ['.ts'],
  only: [path.join(demoDirectory, 'src')],
  presets: [require.resolve('@babel/preset-typescript')],
  plugins: [
    [require.resolve('@isograph/babel-plugin'), { searchFrom: demoDirectory }],
    require.resolve('@babel/plugin-transform-modules-commonjs'),
  ],
});
