import * as fs from 'fs/promises';
import * as path from 'path';
import type { OutputChannel } from 'vscode';
import { getConfig } from '../config';

async function exists(file: string): Promise<boolean> {
  return fs
    .stat(file)
    .then(() => true)
    .catch(() => false);
}

type PlatformBinary =
  | { kind: 'supported'; relativePath: string }
  | { kind: 'unsupported' };

function getBinaryPathRelativeToPackage(): PlatformBinary {
  if (process.platform === 'darwin' && process.arch === 'x64') {
    return {
      kind: 'supported',
      relativePath: path.join('artifacts', 'macos-x64', 'isograph_cli'),
    };
  }

  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return {
      kind: 'supported',
      relativePath: path.join('artifacts', 'macos-arm64', 'isograph_cli'),
    };
  }

  if (process.platform === 'linux' && process.arch === 'x64') {
    return {
      kind: 'supported',
      relativePath: path.join('artifacts', 'linux-x64', 'isograph_cli'),
    };
  }

  if (process.platform === 'linux' && process.arch === 'arm64') {
    return {
      kind: 'supported',
      relativePath: path.join('artifacts', 'linux-arm64', 'isograph_cli'),
    };
  }

  if (process.platform === 'win32' && process.arch === 'x64') {
    return {
      kind: 'supported',
      relativePath: path.join('artifacts', 'win-x64', 'isograph_cli.exe'),
    };
  }

  return { kind: 'unsupported' };
}

async function findIsographCompilerDirectory(
  rootPath: string,
): Promise<string | null> {
  let currentPath = rootPath;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const possiblePackagePath = path.join(
      currentPath,
      'node_modules',
      '@isograph',
      'compiler',
    );

    if (await exists(possiblePackagePath)) {
      return possiblePackagePath;
    }

    const nextPath = path.normalize(path.join(currentPath, '..'));
    if (nextPath === currentPath) {
      break;
    }
    currentPath = nextPath;
  }

  return null;
}

type FindCompiler =
  | { kind: 'found'; path: string }
  | { kind: 'architectureNotSupported' }
  | { kind: 'packageNotFound' };

async function findIsographCompilerBinary(
  rootPath: string,
): Promise<FindCompiler> {
  const isographCompilerDirectory =
    await findIsographCompilerDirectory(rootPath);

  if (isographCompilerDirectory == null) {
    return { kind: 'packageNotFound' };
  }

  const platform = getBinaryPathRelativeToPackage();
  if (platform.kind === 'unsupported') {
    return { kind: 'architectureNotSupported' };
  }

  return {
    kind: 'found',
    path: path.join(isographCompilerDirectory, platform.relativePath),
  };
}

type IsographCompilerBinary = {
  path: string;
};

export async function findIsographBinaryWithWarnings(
  outputChannel: OutputChannel,
  rootPath: string,
): Promise<null | IsographCompilerBinary> {
  const config = getConfig();

  outputChannel.appendLine(JSON.stringify(config));

  if (config.pathToIsograph != null) {
    outputChannel.appendLine(
      `Using isograph.pathToIsograph: ${config.pathToIsograph}`,
    );
    return { path: config.pathToIsograph };
  }

  outputChannel.appendLine(
    `Searching for @isograph/compiler starting at: ${rootPath}`,
  );
  const isographBinaryResult = await findIsographCompilerBinary(rootPath);

  if (isographBinaryResult.kind === 'packageNotFound') {
    outputChannel.appendLine(
      'Could not find @isograph/compiler in node_modules. Set isograph.pathToIsograph to the cargo binary, or install the compiler package.',
    );
    return null;
  }

  if (isographBinaryResult.kind === 'architectureNotSupported') {
    outputChannel.appendLine(
      `@isograph/compiler does not ship a binary for the architecture: ${process.arch}`,
    );
    return null;
  }

  return { path: isographBinaryResult.path };
}
