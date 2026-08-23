# Isograph VSCode extension

Colors `iso(\`...\`)`literals in JavaScript and TypeScript via LSP semantic tokens from`isograph lsp`.

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
