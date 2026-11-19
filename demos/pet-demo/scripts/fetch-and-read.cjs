'use strict';

// Fetches and reads the demo's entrypoints from the mock network, and prints
// each root resolver's result.

const path = require('node:path');

require('./registerBabel.cjs');

require(path.join(__dirname, '..', 'src', 'fetchAndRead'))
  .fetchAndRead()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
