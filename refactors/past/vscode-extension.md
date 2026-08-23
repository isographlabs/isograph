# VS Code extension

Requires lsp-proxy.md and lsp-tokens.md. Independent of lsp-sessions.md, lsp-diagnostics.md, zed-and-vscode-extensions.md. Later: vscode-config-discovery.md replaces workspace-root cwd, `rootDirectory`, and the single client.

`vscode-extension/` is a verbatim copy of isograph `vscode-extension/`. This slice is the i2 extension: spawn `isograph lsp`, color iso literals. It is not a second highlighter. It is not hover, format, completion, or GraphQL tag coloring.

Origin: isograph `vscode-extension/` (the copy already in this tree). Origin of spawn args: `['lsp']` plus optional `--config`. Origin of initialize: vscode-languageclient `LanguageClient`. Delta: drop every feature the server does not have; drop dead code; make `tsc` and prettier succeed; F5 from the repo root.

One shippable change.

## What the user does

From the i2 repo root:

```sh
cd vscode-extension
npm i
npm run esbuild
```

Set `isograph.pathToIsograph` to the absolute path of `target/debug/isograph` (workspace settings of the project that has the config, not of `vscode-extension/`). F5 configuration `Isograph VS Code extension`. In the Extension Development Host, open a project with `isograph.config.json` and a `.ts` file that contains `iso(\`field Pet.fullName { id }\`)`. `field` is a keyword, `Pet` a class, `fullName` and `id` properties.

Output channels `Isograph` and `Isograph LSP Logs` exist.

Unsaved buffer text is not what the daemon colors. Tokens are `lsp_semantic_tokens_for_file` of a `DiskFile`. The watcher interned the last save. `didOpen` / `didChange` are leftover until a later notification arm.

`editor.semanticHighlighting.enabled` defaults on. Do not add a TextMate grammar.

## What this is not

isograph's server advertises hover, format, definition, completion, highlight, code action, execute command, and semantic tokens. vscode-languageclient then registers a VS Code provider for each. i2's server advertises `semanticTokensProvider` only (lsp-tokens.md). Highlighting is the only provider this client registers against our server.

isograph's extension also:

- depends on `GraphQL.vscode-graphql-syntax`, which colors `graphql` tagged templates. `iso(\`...\`)` is a function-call template. That dependency does not color iso literals. Drop it.
- sends `textDocument/formatting` on will-save when `isograph.autoformatIsoLiterals` is true. The server has no formatting arm. Drop the setting and the handler. `autoformatIsoLiterals` is a bool (request formatting / skip). It does not land. A later format slice can add a setting.

vscode-languageclient 9 still puts the kitchen sink in `initialize.capabilities` (hover, completion, formatting, `publishDiagnostics`, `applyEdit`, …). `computeClientCapabilities` always writes `publishDiagnostics` and `applyEdit`. The rest comes from `registerBuiltinFeatures`. There is no public option to send a subset. Overriding `registerBuiltinFeatures` and omitting document sync makes `sendRequest` throw: it reads private `_didChangeTextDocumentFeature`. We do not subclass `LanguageClient`. Extra advertised client capabilities are claims about VS Code, not requests that the server implement those methods.

## Types

Most important first.

```ts
// from vscode-extension/src/config.ts
export type Config = {
  rootDirectory: string | null;
  pathToIsograph: string | null;
  pathToConfig: string | null;
};
```

`string | null` is the VS Code setting schema `["string", "null"]`. Missing is `null`.

```ts
// from vscode-extension/src/context.ts
export type IsographExtensionContext = {
  client: LanguageClient | null;
  lspOutputChannel: OutputChannel;
  extensionContext: ExtensionContext;
  primaryOutputChannel: OutputChannel;
  isographBinaryExecutionOptions: {
    rootPath: string;
    binaryPath: string;
  };
};
```

`client` is `null` until `LanguageClient.start` resolves. Origin `compilerTerminal` and `binaryVersion` have no reader. Drop them.

```ts
// from vscode-extension/src/utils/findIsographBinary.ts
type PlatformBinary =
  | { kind: 'supported'; relativePath: string }
  | { kind: 'unsupported' };

type FindCompiler =
  | { kind: 'found'; path: string }
  | { kind: 'architectureNotSupported' }
  | { kind: 'packageNotFound' };

type IsographCompilerBinary = {
  path: string;
};
```

Origin `FindCompiler` also had `prereleaseCompilerFound` and `versionDidNotMatch`. `isSemverRangeSatisfied` was the literal `true`, so `versionDidNotMatch` was dead. Semver is not a reader. Drop `semver`, those variants, and `binaryVersion`.

The published npm artifact is still `artifacts/<platform>/isograph_cli` (same layout as `@isograph/compiler`). The cargo binary this repo builds is `target/debug/isograph`. Local iteration uses `isograph.pathToIsograph`, not the npm walk.

## `package.json`

Origin `vscode-extension/package.json`. After:

```json
// from vscode-extension/package.json
{
  "name": "isograph",
  "displayName": "Isograph",
  "version": "0.5.3",
  "description": "Isograph-powered IDE experience",
  "repository": {
    "type": "git",
    "url": "https://github.com/isographlabs/isograph",
    "directory": "vscode-extension"
  },
  "license": "MIT",
  "publisher": "isograph",
  "main": "./out/extension.js",
  "categories": ["Programming Languages"],
  "activationEvents": [
    "onLanguage:javascript",
    "onLanguage:javascriptreact",
    "onLanguage:typescript",
    "onLanguage:typescriptreact"
  ],
  "contributes": {
    "configuration": {
      "type": "object",
      "title": "Isograph",
      "properties": {
        "isograph.pathToIsograph": {
          "scope": "workspace",
          "default": null,
          "type": ["string", "null"],
          "description": "Absolute path to the isograph binary. If not provided, the extension will look in the nearest node_modules/@isograph/compiler"
        },
        "isograph.pathToConfig": {
          "scope": "workspace",
          "default": null,
          "type": ["string", "null"],
          "description": "Path to an isograph config relative to rootDirectory. Without this, the compiler searches for your config."
        },
        "isograph.rootDirectory": {
          "scope": "workspace",
          "default": null,
          "type": ["string", "null"],
          "description": "Path relative to the VS Code workspace. Default is the workspace root. Changes where the extension looks for @isograph/compiler and the cwd of isograph lsp (and therefore config walk-up)."
        }
      }
    }
  },
  "scripts": {
    "typecheck": "tsc",
    "prettier-check": "prettier -c .",
    "prettier-write": "prettier --write .",
    "lint": "eslint --max-warnings 0 .",
    "vscode:prepublish": "rimraf tsconfig.tsbuildinfo && rimraf out && npm run esbuild-base -- --minify",
    "build-local": "vsce package",
    "esbuild-base": "esbuild ./src/extension.ts --bundle --outfile=out/extension.js --external:vscode --format=cjs --platform=node",
    "esbuild": "npm run esbuild-base -- --sourcemap",
    "esbuild-watch": "npm run esbuild-base -- --sourcemap --watch"
  },
  "engines": {
    "node": "22.9.0",
    "vscode": "^1.60.0"
  },
  "packageManager": "npm@10.8.3",
  "dependencies": {
    "vscode-languageclient": "^9.0.1"
  },
  "devDependencies": {
    "@types/node": "^17.0.23",
    "@types/vscode": "^1.60.0",
    "@typescript-eslint/eslint-plugin": "^5.13.0",
    "@typescript-eslint/parser": "^5.0.0",
    "@vscode/vsce": "^2.18.0",
    "esbuild": "^0.17.12",
    "eslint": "^8.19.0",
    "eslint-config-airbnb-base": "^15.0.0",
    "eslint-config-airbnb-typescript": "^17.0.0",
    "eslint-plugin-import": "^2.26.0",
    "prettier": "^3.7.4",
    "rimraf": "^5.0.10",
    "typescript": "5.6.3"
  }
}
```

Delta from origin:

- no `extensionDependencies`
- no `contributes.commands`, no `jsonValidation`
- no `isograph.autoformatIsoLiterals`
- no `semver` / `@types/semver`
- `rimraf` is a devDependency (origin scripts call it, origin `package.json` does not list it)
- `esbuild` / `esbuild-watch` pass `--` so `--sourcemap` reaches esbuild. Origin `npm run esbuild-base --sourcemap` is a npm config flag.

Not a pnpm workspace member. `vsce` uses this package's npm. `npm install` in `vscode-extension/` regenerates `package-lock.json`.

## Source

### `config.ts`

Origin `vscode-extension/src/config.ts`. Drop `autoformatIsoLiterals`.

```ts
// from vscode-extension/src/config.ts
import type { ConfigurationScope } from 'vscode';
import { workspace } from 'vscode';

export type Config = {
  rootDirectory: string | null;
  pathToIsograph: string | null;
  pathToConfig: string | null;
};

export function getConfig(scope?: ConfigurationScope): Config {
  const configuration = workspace.getConfiguration('isograph', scope);
  return {
    rootDirectory: configuration.rootDirectory,
    pathToIsograph: configuration.pathToIsograph,
    pathToConfig: configuration.pathToConfig,
  };
}
```

### `context.ts`

Origin `vscode-extension/src/context.ts`. Drop `compilerTerminal` and `binaryVersion`.

```ts
// from vscode-extension/src/context.ts
import type { ExtensionContext, OutputChannel } from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';

export type IsographExtensionContext = {
  client: LanguageClient | null;
  lspOutputChannel: OutputChannel;
  extensionContext: ExtensionContext;
  primaryOutputChannel: OutputChannel;
  isographBinaryExecutionOptions: {
    rootPath: string;
    binaryPath: string;
  };
};
```

### `findIsographBinary.ts`

Origin `vscode-extension/src/utils/findIsographBinary.ts`.

Delta:

- `pathToIsograph` is checked first. Origin walks node_modules then ignores the result when the setting is set.
- no 5000-iteration `throw`. The walk ends when `path.normalize(join(p, '..')) === p`.
- `Promise<T>` type arguments exist. Origin `Promise {` does not typecheck.
- `workspace.workspaceFolders?.[0]?.uri.fsPath` instead of deprecated `workspace.rootPath`.
- log text says `@isograph/compiler`, which is the directory it walks.
- no `window.showErrorMessage`.

```ts
// from vscode-extension/src/utils/findIsographBinary.ts
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
      "Could not find @isograph/compiler in node_modules. Set isograph.pathToIsograph to the cargo binary, or install the compiler package.",
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
```

`exists` is yes/no on `fs.stat`. The walk is a parent walk; a `while` is the state machine.

### `languageClient.ts`

Origin `vscode-extension/src/languageClient.ts`.

Delta:

- no `textDocument/formatting` will-save handler, no `TextEdit` / `WorkspaceEdit` / `Range` / `window` / `TextDocumentIdentifier` imports
- no `markdown: { isTrusted: true }` (no hover markdown)
- no `initializationFailedHandler` that returns `true` (origin retries initialize forever)
- no `killLanguageClient`, no `DidNotError` bool
- `await client.start()` so activate does not return before the handshake

```ts
// from vscode-extension/src/languageClient.ts
import * as path from 'path';
import type { LanguageClientOptions } from 'vscode-languageclient';
import { RevealOutputChannelOn } from 'vscode-languageclient';
import type { ServerOptions } from 'vscode-languageclient/node';
import { LanguageClient } from 'vscode-languageclient/node';
import { getConfig } from './config';
import type { IsographExtensionContext } from './context';

export async function createAndStartLanguageClient(
  context: IsographExtensionContext,
): Promise<void> {
  const config = getConfig();

  context.primaryOutputChannel.appendLine(
    `Using isograph binary: ${context.isographBinaryExecutionOptions.binaryPath}`,
  );

  const args = ['lsp'];

  if (config.pathToConfig != null) {
    args.push('--config');
    args.push(config.pathToConfig);
  }

  const serverOptions: ServerOptions = {
    options: {
      cwd: context.isographBinaryExecutionOptions.rootPath,
    },
    command: path.resolve(
      context.isographBinaryExecutionOptions.rootPath,
      context.isographBinaryExecutionOptions.binaryPath,
    ),
    args,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'javascript' },
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'javascriptreact' },
    ],
    outputChannel: context.lspOutputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Never,
  };

  const client = new LanguageClient(
    'IsographLanguageClient',
    'Isograph Language Client',
    serverOptions,
    clientOptions,
  );

  context.primaryOutputChannel.appendLine(
    `Starting the Isograph Language Server with these options: ${JSON.stringify(
      serverOptions,
    )}`,
  );

  await client.start();
  context.client = client;
}
```

`path.resolve(rootPath, binaryPath)` leaves an absolute `pathToIsograph` unchanged.

### `extension.ts`

Origin `vscode-extension/src/extension.ts`.

Delta:

- `import * as path from 'path'` instead of `import path = require('path')`
- no unused `const config = getConfig()` in `activate`
- `workspaceFolders` instead of `rootPath`
- pass `rootPath` into `findIsographBinaryWithWarnings`
- `await createAndStartLanguageClient`

```ts
// from vscode-extension/src/extension.ts
import * as path from 'path';
import type { ExtensionContext } from 'vscode';
import { window, workspace } from 'vscode';
import { getConfig } from './config';
import type { IsographExtensionContext } from './context';
import { createAndStartLanguageClient } from './languageClient';
import { findIsographBinaryWithWarnings } from './utils/findIsographBinary';

let isographExtensionContext: IsographExtensionContext | null = null;

function workspaceRoot(): string {
  return workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
}

export async function activate(extensionContext: ExtensionContext) {
  isographExtensionContext =
    await buildIsographExtensionContext(extensionContext);

  if (isographExtensionContext != null) {
    isographExtensionContext.primaryOutputChannel.appendLine(
      'Starting the Isograph extension...',
    );

    await createAndStartLanguageClient(isographExtensionContext);
  }
}

async function buildIsographExtensionContext(
  extensionContext: ExtensionContext,
): Promise<IsographExtensionContext | null> {
  const config = getConfig();

  const primaryOutputChannel = window.createOutputChannel('Isograph');
  const lspOutputChannel = window.createOutputChannel('Isograph LSP Logs');

  extensionContext.subscriptions.push(lspOutputChannel);
  extensionContext.subscriptions.push(primaryOutputChannel);

  let rootPath = workspaceRoot();
  if (config.rootDirectory != null) {
    rootPath = path.join(rootPath, config.rootDirectory);
  }

  const binary = await findIsographBinaryWithWarnings(
    primaryOutputChannel,
    rootPath,
  );

  if (binary != null) {
    return {
      client: null,
      extensionContext,
      lspOutputChannel,
      primaryOutputChannel,
      isographBinaryExecutionOptions: {
        rootPath,
        binaryPath: binary.path,
      },
    };
  }

  primaryOutputChannel.appendLine(
    'Stopping execution of the Isograph VSCode extension since we could not find a valid compiler binary.',
  );

  return null;
}

export function deactivate(): Thenable<void> | undefined {
  isographExtensionContext?.primaryOutputChannel.dispose();

  return isographExtensionContext?.client?.stop();
}
```

## Packaging and editor config

Unchanged from origin: `LICENSE.md`, `.eslintrc.js`, `.gitignore`, `.prettierignore`, `.vscode/settings.json`, `tsconfig.json`.

### `.vscodeignore`

Origin has none. vsce then uses `.gitignore`, which contains `out/`, so the vsix can omit `main`. Add:

```
// from vscode-extension/.vscodeignore
.vscode/**
src/**
.gitignore
.eslintrc.js
.prettierrc.json
tsconfig.json
**/*.ts
package-lock.json
```

Do not ignore `out/`.

### `.prettierrc.json`

Origin has none. Origin sources use single quotes. Prettier 3 defaults to double quotes, so `npm run prettier-check` fails on the copy.

```json
// from vscode-extension/.prettierrc.json
{
  "printWidth": 80,
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "all"
}
```

Root prettier must not rewrite this package. Append to `.prettierignore`:

```
# from .prettierignore
vscode-extension/**
```

Root oxlint must not lint it (airbnb eslint is this package's linter). Append to `.oxlintrc.json` `ignorePatterns`: `"vscode-extension"`.

### F5 from the repo root

Origin README requires opening VS Code on `vscode-extension/`. i2 work happens in the repo root.

```json
// from .vscode/tasks.json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "vscode-extension: esbuild",
      "type": "npm",
      "script": "esbuild",
      "path": "vscode-extension/",
      "problemMatcher": []
    }
  ]
}
```

```json
// from .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Isograph VS Code extension",
      "type": "extensionHost",
      "request": "launch",
      "args": [
        "--extensionDevelopmentPath=${workspaceFolder}/vscode-extension"
      ],
      "outFiles": ["${workspaceFolder}/vscode-extension/out/**/*.js"],
      "preLaunchTask": "vscode-extension: esbuild"
    }
  ]
}
```

## README

Origin `vscode-extension/README.md`. After:

````md
// from vscode-extension/README.md
# Isograph VSCode extension

Colors `iso(\`...\`)` literals in JavaScript and TypeScript via LSP semantic tokens from `isograph lsp`.

## How to develop locally

From the i2 repo root:

```sh
cd vscode-extension
npm install
npm run esbuild
```

Set `isograph.pathToIsograph` in the workspace settings of the project that contains `isograph.config.json` to the absolute path of `target/debug/isograph`.

Open the i2 repo in VS Code. Run and Debug: `Isograph VS Code extension`. In the Extension Development Host, open the project that has the config. Open a `.ts` / `.tsx` / `.js` / `.jsx` file.

Output channels: `Isograph` (extension) and `Isograph LSP Logs` (the language client and the `isograph lsp` stdio proxy).

### Settings

These paths are relative to the workspace you open in the Extension Development Host. Set them in that workspace, not in User Settings.

- `isograph.pathToIsograph`: absolute path to the `isograph` binary. For a local cargo build this is `.../i2/target/debug/isograph`. If unset, the extension walks from `rootDirectory` looking for `node_modules/@isograph/compiler` and the platform artifact under `artifacts/`.
- `isograph.pathToConfig`: config path relative to `rootDirectory`, passed as `--config`. If unset, `isograph lsp` walks up from cwd.
- `isograph.rootDirectory`: path relative to the VS Code workspace. Default is the workspace root. cwd of `isograph lsp`, and the start of the node_modules walk.

## Restarting

- New Rust binary: `pnpm watch-rs` at the repo root. Reload the Extension Development Host.
- New extension TypeScript: reload the Extension Development Host. F5 already ran `npm run esbuild`.
````

## `docs-website/docs/development-workflow.md`

Replace the `## VSCode extension` section:

````md
// from docs-website/docs/development-workflow.md
## VSCode extension

### Starting

From the repo root:

```sh
cd vscode-extension
npm i
npm run esbuild
```

Set `isograph.pathToIsograph` to the absolute path of `target/debug/isograph` in the workspace that contains the Isograph config. Run and Debug: `Isograph VS Code extension`. In the Extension Development Host, open that project. The extension starts when you open a JS, JSX, TS, or TSX file.

### Restarting and seeing new changes

- Run `pnpm watch-rs` to rebuild the `isograph` binary.
- Reload the Extension Development Host to use the new binary or a new `out/extension.js`.

### Logs etc.

Show output channel: `Isograph` or `Isograph LSP Logs`. `Isograph` is the extension. `Isograph LSP Logs` is the language client and the `isograph lsp` process. `eprintln` in Rust shows up there.
````

## CI

```yaml
# from .github/workflows/ci.yml
  vscode-extension:
    name: vscode-extension typecheck
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "22.9.0"
      - run: npm ci
        working-directory: vscode-extension
      - run: npm run typecheck
        working-directory: vscode-extension
      - run: npm run lint
        working-directory: vscode-extension
      - run: npm run prettier-check
        working-directory: vscode-extension
```

`all-checks-passed.needs` appends `vscode-extension`.

Do not add `@vscode/test-electron`. Highlighting is lsp-tokens.md. The proxy is lsp-proxy.md. This job proves the Node package typechecks, lints, and formats.

`publish-isograph-extension.yml` already packages `vscode-extension` on v-tags. No change.

## Tests

None in this crate. `npm run typecheck` / `lint` / `prettier-check` in CI. Degenerate cases of the protocol (missing file, empty `data`, `MethodNotFound`) are lsp-tokens.md.

`findIsographBinary` is not unit-tested. A test would mock `vscode` and `fs`. The reader of a missing binary is the output channel and a silent activate.

## Call sites

- VS Code `activate` -> `findIsographBinaryWithWarnings` -> `createAndStartLanguageClient` -> `LanguageClient` stdio `isograph lsp` (`--config` if set)
- `isograph lsp` -> `{slug}.port` -> `semanticTokens/full` (lsp-tokens.md)
- deactivate -> `client.stop()` -> proxy stdin EOF -> proxy exit -> session `Drop`
- Zed is zed-and-vscode-extensions.md
