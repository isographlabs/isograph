# Isograph VSCode extension

## How to develop locally

- From the command line:

```sh
fnm install --resolve-engines
fnm use --resolve-engines
cd vscode-extension
yarn
yarn build-local
```

- Then, open up VSCode to the vscode-extension folder. The following step will not work if you open VSCode to a different folder.
- Open the Run & Debug sidebar (Cmd + Shift + D), and click Run and Debug.
- This should open something named Extension Development Host. From here, open the folder containing the Isograph config, which should be at the root of your project.
- Now, you should be able to see `Isograph` and `Isograph LSP Logs` in the output pane.

### Settings

:::note
These settings contain relative directories. Thus, you probably want to set these in Workspace Settings, not User Settings.
:::

- `isograph.pathToConfig`: defaults to the workspace root + `/isograph.config.json`, but you may specify a different path.
- `isograph.pathToCompiler`: defaults to `node_modules/isograph-compiler/bin/...`. If you are testing the compiler on a demo and building locally, you likely want to set this to `../../target/debug/isograph_cli`
- `isograph.rootDirectory`: If you open a different folder, e.g. `~/isograph`, and the config is in a subfolder, use this config option.

## Restarting, etc

- Restarting with a new Rust binary:
  - You should run `watch-rs` in a terminal, so that you get the latest binary, if you are also iterating on the Rust code.
  - Reloading the extension development host VSCode window will restart the extension with the latest binary.
- Restarting with changes to the VSCode extension:
  - Run `yarn build-local` and restart the extension development host VSCode window.
