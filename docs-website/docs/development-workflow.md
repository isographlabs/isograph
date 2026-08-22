# Development workflow

## Overview

There are two major places to make changes to Isograph:

- to the Rust compiler, and thus to generated files
- to the Rust language server

## Global setup

Install [`mise`](https://mise.jdx.dev/), activate it in your shell (`eval "$(mise activate zsh)"` or the equivalent for your shell), then from the repository root:

```sh
mise trust
mise install
mise doctor
```

`mise.toml` pins Node, pnpm, and bacon. `package.json` `engines.node` and `packageManager` must match those pins. `.node-version` matches the Node pin.

Rust is still latest stable via rustup. CI installs it with `actions-rust-lang/setup-rust-toolchain`. bacon versions newer than 3.6 will not work with our bacon.toml. (This is fixable, but we have not needed to.)

## Commands related to the compiler and Rust

### Building the compiler

```sh
pnpm watch-rs
```

This will watch and rebuild the compiler for use locally.

:::info
This starts bacon! There are several commands you can press: `b` to build, `c` for running clippy, `a` for checking. It defaults to building.
:::

### Running the compiler binary directly

The compiler can be run with `./target/debug/isograph`. If you are using the locally-built compiler from another folder, you should be able to run `$PATH_TO_ISOGRAPH_REPO/target/debug/isograph --config $YOUR_LOCAL_CONFIG`.

### Running the compiler in a project where `@isograph/compiler` was installed via `yarn`

```sh
yarn run iso
```

### Running Rust tests

```sh
cargo test
```

(These are not run as part of CI, but we should add that!)

### Format Rust code

```sh
pnpm format-rust
# or
cargo fmt
# or
pnpm format # which also formats the TypeScript code
```

Many of these tests come from the libraries that we brought in from Relay, and aren't specific to Isograph.

### Documentation

To show the rustdoc, `pnpm watch-rs`, then press the `d` key.

## Commands related to JavaScript

### Install dependencies

You can install everything by running the following from the root:

```sh
pnpm i
```

### Format the code

```sh
pnpm format-prettier
# or
pnpm format # which also formats the Rust code
```

## VSCode extension

### Starting

- Open VSCode in `isograph/vscode-extension`
- Run the following in `isograph/vscode-extension`:

```sh
npm i
npm run build-local
```

- Open `src/extension.ts` in your editor, then open the "run and debug" sidebar and click `Run and Debug`. If given a choice, select something related to "Extension development host".
- The VSCode extension should start when you open a JS, JSX, TS or TSX file.

### Restarting and seeing new changes

- Run `pnpm watch-rs` to ensure that the latest binary is being built
- Restart the "Extension development host" window to use the latest language server binary.

### Logs etc.

You can see logs by going to `Show output channel` and selecting `Isograph` or `Isograph LSP Logs`. `Isograph` is the output of the VSCode extension. It is not very interesting. `Isograph LSP Logs` shows the output of the language server binary and the traffic. This is interesting. `eprintln`'s in your Rust code will show up here.

## How to release a new "main" version of Isograph

Every commit to `main` results in a build, which you can see in [npm](https://www.npmjs.com/package/@isograph/compiler?activeTab=versions). The ones of the form `0.0.0-main-$hash` are generated from a commit to `main`.

## How to release a new "numbered" version of Isograph to npm

- In all package.json files, bump the version number. Don't forget to bump the version number of imports.
- `pnpm i`
- `git add . && git commit -m 'v0.1.0' && git tag v0.1.0 && git push && git push --tags`
- See [this commit releasing 0.2.0](https://github.com/isographlabs/isograph/commit/e36acab1a018e18bdae0558be08952693af3b6a8)

## Workflow for using Isograph

If you are using Isograph in a project, you may be interested in [this doc](../workflow).

## Checking things that fail in CI locally

You may save yourself some time by running:

```sh
pnpm sanity-check
```

This will format the code, run clippy, and ensure that no files are left modified in the working directory.
