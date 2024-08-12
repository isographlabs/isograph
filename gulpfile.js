const path = require('path');
const fs = require('fs');

const RELEASE_COMMIT_SHA = process.env.RELEASE_COMMIT_SHA;
const VERSION = RELEASE_COMMIT_SHA
  ? `0.0.0-main-${RELEASE_COMMIT_SHA.substr(0, 8)}`
  : process.env.npm_package_version;

const builds = [
  {
    folder: 'isograph-babel-plugin',
    packageName: '@isograph/babel-plugin',
    hasJsr: false,
  },
  {
    folder: 'isograph-disposable-types',
    packageName: '@isograph/disposable-types',
    hasJsr: true,
  },
  {
    folder: 'isograph-react',
    packageName: '@isograph/react',
    hasJsr: true,
  },
  {
    folder: 'isograph-react-disposable-state',
    packageName: '@isograph/react-disposable-state',
    hasJsr: true,
  },
  {
    folder: 'isograph-reference-counted-pointer',
    packageName: '@isograph/reference-counted-pointer',
    hasJsr: true,
  },
  {
    folder: 'isograph-compiler',
    packageName: '@isograph/compiler',
    hasJsr: false,
  },
];

const setMainVersion = async () => {
  if (!RELEASE_COMMIT_SHA) {
    throw new Error('Expected the RELEASE_COMMIT_SHA env variable to be set.');
  }

  const packages = builds.map((build) => build.packageName);
  builds.forEach((build) => {
    // package.json
    const packageJsonPath = path.join(
      '.',
      'libs',
      build.folder,
      'package.json',
    );
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    packageJson.version = VERSION;
    for (const depKind of [
      'dependencies',
      'devDependencies',
      'peerDependencies',
    ]) {
      const deps = packageJson[depKind];
      for (const dep in deps) {
        if (packages.includes(dep)) {
          deps[dep] = VERSION;
        }
      }
    }
    fs.writeFileSync(
      packageJsonPath,
      JSON.stringify(packageJson, null, 2) + '\n',
      'utf8',
    );

    // jsr.json
    if (build.hasJsr) {
      const jsrJsonPath = path.join('.', 'libs', build.folder, 'jsr.json');
      const jsrJson = JSON.parse(fs.readFileSync(jsrJsonPath, 'utf8'));
      jsrJson.version = VERSION;

      if (jsrJson.imports != null) {
        const newImports = {};
        const imports = jsrJson.imports;
        Object.keys(imports).forEach((importName) => {
          if (importName.contains('isograph')) {
            // TODO remove testscope
            newImports[importName] =
              `jsr:${importName.replace('/', 'testscope/')}@${VERSION}`;
          } else {
            newImports[importName] = imports[importName];
          }
        });
        jsrJson.imports = newImports;
      }

      fs.writeFileSync(
        jsrJsonPath,
        JSON.stringify(jsrJson, null, 2) + '\n',
        'utf8',
      );
    }
  });
};

exports.setMainVersion = setMainVersion;
