# VS Code config discovery

Requires vscode-extension.md. Independent of lsp-sessions.md, lsp-diagnostics.md, zed-and-vscode-extensions.md.

vscode-extension.md starts one `LanguageClient`. cwd is the workspace root (or `isograph.rootDirectory`). `isograph lsp` walk-up starts there. A config in a nested package is not found. Two configs in one window cannot both be served: one process per config (event-model.md), one stdio proxy per process.

This slice finds the same files `discover.rs` `nearest_config` finds, starts one client per canonical path, and gives each client a document selector that does not overlap another config.

Origin of the walk: `crates/isograph_cli/src/discover.rs` `CONFIG_FILE_NAMES` / `nearest_config`. Origin of the client: vscode-extension.md `createAndStartLanguageClient`. Delta: walk from the file and from `workspace.findFiles`, always pass `--config`, drop `rootDirectory`, N clients.

One shippable change.

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
// from vscode-extension/src/discover.ts
export type Entry =
  | { kind: 'file'; name: string }
  | { kind: 'directory'; name: string };

export type Glob = {
  base: string;
  pattern: string;
};
```

`ownedGlobs` reads a directory listing through `entries` so tests pass a tree, not `fs`.

```ts
// from vscode-extension/src/context.ts
export type Session = {
  configPath: string;
  configDir: string;
  globs: Glob[];
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

`configPath` is canonical (`fs.realpath`). `configDir` is `path.dirname(configPath)`. `binaryPath` is `isograph.pathToIsograph` when set, otherwise null and each session walks from its `configDir`. Origin `isographBinaryExecutionOptions.rootPath` was the workspace cwd hack. Drop it.

```ts
// from vscode-extension/src/config.ts
export type Config = {
  pathToIsograph: string | null;
  pathToConfig: string | null;
};
```

`rootDirectory` is gone. It existed to point walk-up at a nested folder. The walk is from the file.

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

export type Entry =
  | { kind: 'file'; name: string }
  | { kind: 'directory'; name: string };

export type Glob = {
  base: string;
  pattern: string;
};

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

export function isStrictDescendant(child: string, parent: string): boolean {
  const rel = path.relative(path.resolve(parent), path.resolve(child));
  return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
}

export function nearestDiscoveredConfig(
  filePath: string,
  configDirs: readonly string[],
): string | null {
  const fileDir = path.dirname(path.resolve(filePath));
  const matches = configDirs.filter(
    (configDir) =>
      path.resolve(configDir) === fileDir ||
      isStrictDescendant(fileDir, configDir),
  );
  matches.sort((a, b) => b.length - a.length);
  return matches[0] ?? null;
}

export async function ownedGlobs(
  configDir: string,
  nestedConfigDirs: readonly string[],
  entries: (dir: string) => Promise<readonly Entry[]>,
): Promise<Glob[]> {
  const resolved = path.resolve(configDir);
  const nested = nestedConfigDirs
    .map((p) => path.resolve(p))
    .filter((p) => isStrictDescendant(p, resolved));
  return globsFrom(resolved, nested, entries);
}

async function globsFrom(
  dir: string,
  nested: readonly string[],
  entries: (dir: string) => Promise<readonly Entry[]>,
): Promise<Glob[]> {
  const hasNestedHere = nested.some(
    (n) => n === dir || isStrictDescendant(n, dir),
  );
  if (!hasNestedHere) {
    return [{ base: dir, pattern: '**/*' }];
  }
  if (nested.includes(dir)) {
    return [];
  }
  const listing = await entries(dir);
  const fromChildren = await Promise.all(
    listing
      .filter((e) => e.kind === 'directory')
      .map((e) => globsFrom(path.join(dir, e.name), nested, entries)),
  );
  return [{ base: dir, pattern: '*' }, ...fromChildren.flat()];
}

export async function readEntries(dir: string): Promise<readonly Entry[]> {
  const dirents = await fs.readdir(dir, { withFileTypes: true });
  return dirents.map((d) =>
    d.isDirectory()
      ? { kind: 'directory', name: d.name }
      : { kind: 'file', name: d.name },
  );
}
```

`nearestConfig` is `nearest_config` plus `canonicalize`. A directory named `isograph.config.json` is not a file; the next name in that directory is tried. `isFile` is yes/no on `stat`. The walk is a parent walk.

`ownedGlobs` is why two clients do not both register semantic tokens on one document. vscode `DocumentFilter` has no exclude. If config B's directory is inside config A's, A cannot use `A/**`. A gets `*` in A plus `**/*` on sibling trees; B gets `B/**`.

`nearestDiscoveredConfig` is which of the already-started sessions owns a file (longest matching `configDir`). It does not walk the filesystem.

## Tests

`vscode-extension/src/discover.test.ts`. `node:test` and `node:assert/strict`. No `vscode`. `tsc` emits `out/discover.test.js`. esbuild entry is `extension.ts`; tests are not in the bundle. `.vscodeignore` appends `out/**/*.test.js`.

`@types/node` becomes `22.9.0` (engines.node). Origin `^17` has no `node:test`.

`package.json` script `"test": "tsc && node --test out/discover.test.js"`. The vscode-extension CI job runs `npm test` after `typecheck` (or instead: `test` already runs `tsc`). Keep `typecheck` and add `test`.

Cases for `nearestConfig` (same facts as `discover.rs`):

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

Cases for `ownedGlobs` (in-memory `entries`):

- no nested: `[{ base: configDir, pattern: '**/*' }]`
- sibling nested configs are not descendants: each still `**/*`
- `configDir=/repo`, nested=`/repo/apps/web`, listing `/repo` has `apps`, `src`; `/repo/apps` has `web`, `admin`:
  - `{ base: '/repo', pattern: '*' }`
  - `{ base: '/repo/src', pattern: '**/*' }`
  - `{ base: '/repo/apps', pattern: '*' }`
  - `{ base: '/repo/apps/admin', pattern: '**/*' }`
  - nothing under `/repo/apps/web`
- nested equal to `configDir` is not a strict descendant: `**/*`
- empty listing and a nested child only: `{ base: dir, pattern: '*' }` plus empty from that child

Cases for `nearestDiscoveredConfig`:

- file under the deeper of two ancestor configs returns the deeper
- file under none returns null
- file in the config directory itself returns that config

`isStrictDescendant('/a', '/a')` is false. `isStrictDescendant('/a/b', '/a')` is true. `isStrictDescendant('/a', '/a/b')` is false. `isStrictDescendant('/a/c', '/a/b')` is false.

## Client

```ts
// from vscode-extension/src/languageClient.ts
export async function createAndStartLanguageClient(options: {
  binaryPath: string;
  configPath: string;
  configDir: string;
  globs: Glob[];
  lspOutputChannel: OutputChannel;
  primaryOutputChannel: OutputChannel;
}): Promise<LanguageClient> {
  const languages = [
    'javascript',
    'typescript',
    'typescriptreact',
    'javascriptreact',
  ];
  const documentSelector = options.globs.flatMap((glob) =>
    languages.map((language) => ({
      scheme: 'file',
      language,
      pattern: new RelativePattern(glob.base, glob.pattern),
    })),
  );

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

Always `--config` with the canonical path. cwd is the config directory (relative paths in the config). `pathToConfig` from settings is not read here; `extension.ts` resolved it already.

`RelativePattern` is `vscode.RelativePattern`.

When `pathToConfig` is set, `globs` is `[{ base: workspaceFolder, pattern: '**/*' }]` for each workspace folder (one daemon, whole window). Nested configs in that window are not started.

## `extension.ts`

One `IsographExtensionContext` with `sessions: []`. Activate:

1. Output channels as today.
2. If `pathToIsograph` is set, `binaryPath` is that. Else `binaryPath` is null and each session calls `findIsographBinaryWithWarnings(channel, configDir)`.
3. If `pathToConfig` is set, resolve it (absolute, or relative to `workspace.workspaceFolders[0]`), `realpath`, start one session, return.
4. Else `workspace.findFiles('**/isograph.config.{json,js,ts}', '**/{node_modules,.git}/**')`, `realpath` each, unique. Start a session per path. `ownedGlobs(configDir, otherConfigDirs, readEntries)`.
5. `workspace.textDocuments` and `workspace.onDidOpenTextDocument`: skip non-`file` scheme. `nearestConfig(dirname(uri.fsPath))`. If null, log and skip. If a session for that path exists, `ensureGlobsCover(file)` (recompute `ownedGlobs` and restart that client if the glob list changed). If not, start a session; recompute globs for ancestor sessions (their nested set grew) and restart those whose globs changed.
6. `createFileSystemWatcher('**/isograph.config.{json,js,ts}')` on create and delete: run step 4+5 again (stop sessions whose path vanished, start new, recompute remaining).

`deactivate` stops every `session.client`.

Starting a session with `binaryPath == null` and no package under `configDir` logs and does not push a session. Other configs still start.

Restart is `client.stop()` then `createAndStartLanguageClient` with the new globs, replace `session.client` and `session.globs`.

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

vscode-extension.md already takes `rootPath` as the walk start. Callers pass `configDir`. No further delta except `rootDirectory` is gone from `getConfig`, so this file does not read it (it already does not, after vscode-extension.md).

## Call sites

- `activate` / `findFiles` / `didOpen` / config watcher -> `nearestConfig` / `ownedGlobs` -> `createAndStartLanguageClient` -> `isograph lsp --config <canonical>`
- two sessions, two proxies, two daemons when two configs
- `pathToConfig` set -> one session, findFiles skipped
- Zed is unchanged (worktree cwd). zed-and-vscode-extensions.md
