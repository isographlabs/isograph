import * as path from 'path';
import * as fs from 'fs/promises';
import * as semver from 'semver';
import { OutputChannel, window, workspace } from 'vscode';
import { getConfig } from '../config';

async function exists(file: string): Promise<boolean> {
  return fs
    .stat(file)
    .then(() => true)
    .catch(() => false);
}

// This is derived from the isograph-compiler npm package
function getBinaryPathRelativeToPackage(): string | null {
  if (process.platform === 'darwin' && process.arch === 'x64') {
    return path.join('macos-x64', 'isograph');
  }

  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return path.join('macos-arm64', 'isograph');
  }

  if (process.platform === 'linux' && process.arch === 'x64') {
    return path.join('linux-x64', 'isograph');
  }

  if (process.platform === 'linux' && process.arch === 'arm64') {
    return path.join('linux-arm64', 'isograph');
  }

  if (process.platform === 'win32' && process.arch === 'x64') {
    return path.join('win-x64', 'isograph.exe');
  }

  return null;
}

async function findIsographCompilerDirectory(
  rootPath: string,
): Promise<string | null> {
  let counter = 0;
  let currentPath = rootPath;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (counter >= 5000) {
      throw new Error(
        'Could not find isograph-compiler directory after 5000 traversals. This is likely a bug in the extension code and should be reported to https://github.com/isographlabs/isograph/issues',
      );
    }

    counter += 1;

    const possibleBinaryPath = path.join(
      currentPath,
      'node_modules',
      'isograph-compiler',
    );

    if (await exists(possibleBinaryPath)) {
      return possibleBinaryPath;
    }

    const nextPath = path.normalize(path.join(currentPath, '..'));

    // Eventually we'll get to `/` and get stuck in a loop.
    if (nextPath === currentPath) {
      break;
    } else {
      currentPath = nextPath;
    }
  }

  return null;
}

type IsographCompilerPackageInformation =
  | { kind: 'compilerFound'; path: string; version: string }
  | { kind: 'prereleaseCompilerFound'; path: string; version: string }
  | { kind: 'architectureNotSupported' }
  | { kind: 'packageNotFound' }
  | {
      kind: 'versionDidNotMatch';
      path: string;
      version: string;
      expectedRange: string;
    };

async function findIsographCompilerBinary(
  rootPath: string,
): Promise<IsographCompilerPackageInformation> {
  const isographCompilerDirectory =
    await findIsographCompilerDirectory(rootPath);

  if (!isographCompilerDirectory) {
    return { kind: 'packageNotFound' };
  }

  const isographBinaryRelativeToPackage = getBinaryPathRelativeToPackage();

  if (!isographBinaryRelativeToPackage) {
    return { kind: 'architectureNotSupported' };
  }

  const packageManifest = JSON.parse(
    await fs.readFile(
      path.join(isographCompilerDirectory, 'package.json'),
      'utf-8',
    ),
  );

  const isSemverRangeSatisfied = true;

  // If you are using a pre-release version of the compiler, we assume you know
  // what you are doing.
  const isPrerelease = semver.prerelease(packageManifest.version) != null;

  const isographBinaryPath = path.join(
    isographCompilerDirectory,
    isographBinaryRelativeToPackage,
  );

  if (isPrerelease) {
    return {
      kind: 'prereleaseCompilerFound',
      path: isographBinaryPath,
      version: packageManifest.version,
    };
  }
  if (isSemverRangeSatisfied) {
    return {
      kind: 'compilerFound',
      path: isographBinaryPath,
      version: packageManifest.version,
    };
  }

  return {
    kind: 'versionDidNotMatch',
    path: isographBinaryPath,
    expectedRange: '*',
    version: packageManifest.version,
  };
}

type IsographCompilerBinary = {
  /**
   * The path to the binary.
   */
  path: string;
  /**
   * The version of the binary, or `undefined` if the binary
   * wasn't resolved through the versioned isograph-compiler package.
   */
  version?: string;
};

export async function findIsographBinaryWithWarnings(
  outputChannel: OutputChannel,
): Promise<IsographCompilerBinary | null> {
  const config = getConfig();

  let rootPath = workspace.rootPath || process.cwd();
  if (config.rootDirectory) {
    rootPath = path.join(rootPath, config.rootDirectory);
  }

  outputChannel.appendLine(JSON.stringify(config));
  outputChannel.appendLine(
    `Searching for the isograph-compiler starting at: ${rootPath}`,
  );
  const isographBinaryResult = await findIsographCompilerBinary(rootPath);

  if (config.pathToIsograph) {
    outputChannel.appendLine(
      "You've manually specified 'isograph.pathToBinary'. We cannot confirm this version of the Isograph Compiler is supported by this version of the extension. I hope you know what you're doing.",
    );

    return { path: config.pathToIsograph };
  }
  if (isographBinaryResult.kind === 'versionDidNotMatch') {
    window.showErrorMessage(
      // Array syntax so it's easier to read this message in the source code.
      [
        `The installed version of the Isograph Compiler is version: '${isographBinaryResult.version}'.`,
        `We found this version in the package.json at the following path: ${isographBinaryResult.path}`,
        `This version of the extension supports the following semver range: '${isographBinaryResult.expectedRange}'.`,
        'Please update your extension / isograph-compiler to accommodate the version requirements.',
      ].join(' '),
      'Okay',
    );

    return null;
  }
  if (isographBinaryResult.kind === 'packageNotFound') {
    outputChannel.appendLine(
      "Could not find the 'isograph-compiler' package in your node_modules. Maybe you're not inside of a project with isograph installed.",
    );

    return null;
  }
  if (isographBinaryResult.kind === 'architectureNotSupported') {
    outputChannel.appendLine(
      `The 'isograph-compiler' does not ship a binary for the architecture: ${process.arch}`,
    );

    return null;
  }
  if (isographBinaryResult.kind === 'prereleaseCompilerFound') {
    outputChannel.appendLine(
      [
        'You have a pre-release version of the isograph-compiler package installed.',
        'We are unable to confirm if this version is compatible with the Isograph',
        'VSCode Extension. Proceeding on the assumption that you know what you are',
        'doing.',
      ].join(' '),
    );

    return {
      path: isographBinaryResult.path,
      version: isographBinaryResult.version,
    };
  }

  if (isographBinaryResult.kind === 'compilerFound') {
    return {
      path: isographBinaryResult.path,
      version: isographBinaryResult.version,
    };
  }

  return null;
}
