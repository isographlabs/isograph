'use strict';

// Compiles the demo's sources with the Isograph Babel plugin as Node requires
// them, then requires every module that has an entrypoint literal and logs the
// artifact its exported entrypoint literal is replaced with, with the reader
// artifact its `readerArtifact` function returns.

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

for (const moduleName of [
  'HomeRoute',
  'PetDetailRoute',
  'SpeciesRoute',
  'SetTagline',
]) {
  const entrypoint = require(path.join(sourceDirectory, moduleName))[
    `${moduleName}Entrypoint`
  ];
  console.log({ ...entrypoint, readerArtifact: entrypoint.readerArtifact() });
}
