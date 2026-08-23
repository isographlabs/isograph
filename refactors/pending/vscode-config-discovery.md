# VS Code config discovery

Requires vscode-extension.md. Independent of lsp-sessions.md, lsp-diagnostics.md, zed-and-vscode-extensions.md.

The config is not always at the workspace root. There may be several. vscode-extension.md starts one `LanguageClient` whose cwd is the workspace root (or `isograph.rootDirectory`). `isograph lsp` walk-up starts there. A nested `apps/web/isograph.config.json` is not found. A second config in the same window has no client.

This slice finds every `isograph.config.json` / `.js` / `.ts` in the workspace (and, on open, by walking up from the file), and starts one client per canonical path.

Origin of the walk: `crates/isograph_cli/src/discover.rs` `CONFIG_FILE_NAMES` / `nearest_config`. Origin of the client: vscode-extension.md `createAndStartLanguageClient`. Delta: search instead of assuming the repo root; N clients; always `--config`; drop `rootDirectory`.

One shippable change.

A config directory that contains another config directory is two `**/*` selectors on the inner files. This slice does not pick a winner. Sibling packages (`apps/web` and `apps/admin`) do not overlap.

## What the user does

Open the monorepo root. Do not set `isograph.rootDirectory` or `isograph.pathToConfig`.

```
repo/
  apps/web/isograph.config.json
  apps/web/src/Home.ts
  apps/admin/isograph.config.json
  apps/admin/src/App.ts
```

Open `Home.ts`. `field` in that file is a keyword. Open `App.ts`. Same, from the admin daemon. Output channel `Isograph` logs both canonical config paths.

`isograph.pathToConfig` still exists. When it is set, the window has one client for that file and does not search.

## Types

Most important first.

```ts
// from vscode-extension/src/discover.ts
export const CONFIG_FILE_NAMES = [
  'isograph.config.json',
  'isograph.config.js',
  'isograph.config.ts',
] as const;
```

Same names, same order as `discover.rs`. Copied. A third name is a change to both files.

```ts
// from vscode-extension/src/context.ts
export type Session = {
  configPath: string;
  configDir: string;
  client: LanguageClient;
};

export type IsographExtensionContext = {
  sessions: Session[];
  lspOutputChannel: OutputChannel;
  extensionContext: ExtensionContext;
  primaryOutputChannel: OutputChannel;
  binaryPath: string | null;
};
```

`configPath` is canonical (`fs.realpath`). `configDir` is `path.dirname(configPath)`. `binaryPath` is `isograph.pathToIsograph` when set, otherwise null and each session walks from its `configDir`. Origin `isographBinaryExecutionOptions.rootPath` was the workspace cwd. Drop it.

```ts
// from vscode-extension/src/config.ts
export type Config = {
  pathToIsograph: string | null;
  pathToConfig: string | null;
};
```

`rootDirectory` is gone. It existed to point walk-up at a nested folder.

## `discover.ts`

New file. No `vscode` import.

```ts
// from vscode-extension/src/discover.ts
import * as fs from 'fs/promises';
import * as path from 'path';

export const CONFIG_FILE_NAMES = [
  'isograph.config.json',
  'isograph.config.js',
  'isograph.config.ts',
] as const;

async function isFile(candidate: string): Promise<boolean> {
  try {
    const stat = await fs.stat(candidate);
    return stat.isFile();
  } catch {
    return false;
  }
}

export async function nearestConfig(start: string): Promise<string | null> {
  let dir = path.resolve(start);
  while (true) {
    for (const name of CONFIG_FILE_NAMES) {
      const candidate = path.join(dir, name);
      if (await isFile(candidate)) {
        try {
          return await fs.realpath(candidate);
        } catch {
          return null;
        }
      }
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}
```

`nearestConfig` is `nearest_config` plus `canonicalize`. A directory named `isograph.config.json` is not a file; the next name in that directory is tried. `isFile` is yes/no on `stat`. The walk is a parent walk.

## Tests

`vscode-extension/src/discover.test.ts`. `node:test` and `node:assert/strict`. No `vscode`. `tsc` emits `out/discover.test.js`. esbuild entry is `extension.ts`; tests are not in the bundle. `.vscodeignore` appends `out/**/*.test.js`.

`@types/node` becomes `22.9.0` (engines.node). Origin `^17` has no `node:test`.

`package.json` script `"test": "tsc && node --test out/discover.test.js"`. The vscode-extension CI job runs `npm test` after `typecheck`. Keep `typecheck` and add `test`.

Cases (same facts as `discover.rs` `nearest_config_*`):

- json from a nested directory
- js when there is no json
- ts when there is no json or js
- json preferred to js in the same directory
- json preferred to ts in the same directory
- js preferred to ts in the same directory
- none when the tree has no config
- walks up to a parent
- child js preferred to a parent json
- directory named `isograph.config.json` is skipped
- sibling directory's config is ignored
- realpath of a dotted path matches the real file

## Client

```ts
// from vscode-extension/src/languageClient.ts
export async function createAndStartLanguageClient(options: {
  binaryPath: string;
  configPath: string;
  configDir: string;
  lspOutputChannel: OutputChannel;
  primaryOutputChannel: OutputChannel;
}): Promise<LanguageClient> {
  const documentSelector = [
    'javascript',
    'typescript',
    'typescriptreact',
    'javascriptreact',
  ].map((language) => ({
    scheme: 'file',
    language,
    pattern: new RelativePattern(options.configDir, '**/*'),
  }));

  const args = ['lsp', '--config', options.configPath];

  const serverOptions: ServerOptions = {
    options: { cwd: options.configDir },
    command: options.binaryPath,
    args,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector,
    outputChannel: options.lspOutputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Never,
  };

  const client = new LanguageClient(
    `isograph-${options.configPath}`,
    `Isograph (${options.configPath})`,
    serverOptions,
    clientOptions,
  );

  options.primaryOutputChannel.appendLine(
    `Starting isograph lsp --config ${options.configPath}`,
  );

  await client.start();
  return client;
}
```

Always `--config` with the canonical path. cwd is the config directory. `RelativePattern` is `vscode.RelativePattern`. Selector is that config's tree. Sibling configs do not overlap.

When `pathToConfig` is set, one client, selector is the four languages with no pattern (the whole window). Search is skipped.

## `extension.ts`

One `IsographExtensionContext` with `sessions: []`. Activate:

1. Output channels as today.
2. If `pathToIsograph` is set, `binaryPath` is that. Else `binaryPath` is null and each session calls `findIsographBinaryWithWarnings(channel, configDir)`.
3. If `pathToConfig` is set, resolve it (absolute, or relative to `workspace.workspaceFolders[0]`), `realpath`, start one session, return.
4. Else `workspace.findFiles('**/isograph.config.{json,js,ts}', '**/{node_modules,.git}/**')`, `realpath` each, unique. Start a session per path.
5. `workspace.textDocuments` and `workspace.onDidOpenTextDocument`: skip non-`file` scheme. `nearestConfig(dirname(uri.fsPath))`. If null, log and skip. If no session for that path, start one.
6. `createFileSystemWatcher('**/isograph.config.{json,js,ts}')` on create and delete: start a session for a new path; stop a session whose path vanished.

`deactivate` stops every `session.client`.

Starting a session with `binaryPath == null` and no package under `configDir` logs and does not push a session. Other configs still start.

```ts
// from vscode-extension/src/extension.ts
function workspaceRoot(): string {
  return workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
}
```

Used only to resolve a relative `pathToConfig`. Not used as cwd for lsp.

## Settings

`package.json` `isograph.rootDirectory` is removed.

`isograph.pathToConfig` description: absolute path, or relative to the first workspace folder. When set, no search. When unset, search.

`isograph.pathToIsograph` unchanged. When unset, the node_modules walk starts at that session's `configDir`, not the workspace root.

README and `docs-website/docs/development-workflow.md`: drop `rootDirectory`. Nested project: open the monorepo, do not set a config path. Local cargo binary still uses `pathToIsograph`.

## `findIsographBinary.ts`

vscode-extension.md already takes `rootPath` as the walk start. Callers pass `configDir`.

## Call sites

- `activate` / `findFiles` / `didOpen` / config watcher -> `nearestConfig` -> `createAndStartLanguageClient` -> `isograph lsp --config <canonical>`
- two sessions, two proxies, two daemons when two configs
- `pathToConfig` set -> one session, findFiles skipped
- Zed is unchanged (worktree cwd). zed-and-vscode-extensions.md
