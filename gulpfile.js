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
  },
  {
    folder: 'isograph-disposable-types',
    packageName: '@isograph/disposable-types',
  },
  {
    folder: 'isograph-react',
    packageName: '@isograph/react',
  },
  {
    folder: 'isograph-react-disposable-state',
    packageName: '@isograph/react-disposable-state',
  },
  {
    folder: 'isograph-reference-counted-pointer',
    packageName: '@isograph/reference-counted-pointer',
  },
  {
    folder: 'isograph-compiler',
    packageName: '@isograph/compiler',
  },
];

const setMainVersion = async () => {
  if (!RELEASE_COMMIT_SHA) {
    throw new Error('Expected the RELEASE_COMMIT_SHA env variable to be set.');
  }

  const packages = builds.map((build) => build.packageName);
  builds.forEach((build) => {
    const pkgJsonPath = path.join('.', 'libs', build.folder, 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
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
      pkgJsonPath,
      JSON.stringify(packageJson, null, 2) + '\n',
      'utf8',
    );
  });
};

exports.setMainVersion = setMainVersion;
