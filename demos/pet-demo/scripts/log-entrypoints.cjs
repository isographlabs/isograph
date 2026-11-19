'use strict';

// Requires every module that has an entrypoint literal and logs the artifact
// its exported entrypoint literal is replaced with, with the reader artifact
// its `readerArtifact` function returns.

const path = require('node:path');

require('./registerBabel.cjs');

const sourceDirectory = path.join(__dirname, '..', 'src');

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
