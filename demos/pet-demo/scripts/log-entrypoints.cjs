'use strict';

// Compiles the demo's sources with the Isograph Babel plugin as Node requires
// them, then requires every module that has an entrypoint literal. Each of
// those modules logs the artifact its entrypoint literal is replaced with.

const path = require('node:path');

const demoDirectory = path.join(__dirname, '..');
const sourceDirectory = path.join(demoDirectory, 'src');

require('@babel/register')({
  babelrc: false,
  configFile: false,
  cache: false,
  extensions: ['.ts'],
  only: [sourceDirectory],
  presets: [require.resolve('@babel/preset-typescript')],
  plugins: [
    [require.resolve('@isograph/babel-plugin'), { searchFrom: demoDirectory }],
    require.resolve('@babel/plugin-transform-modules-commonjs'),
  ],
});

['HomeRoute', 'PetDetailRoute', 'SpeciesRoute', 'SetTagline'].forEach(
  (moduleName) => require(path.join(sourceDirectory, moduleName)),
);
