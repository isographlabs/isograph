# Development workflow

## Overview

There are three major places to make changes to Isograph:

- to the Rust compiler, and thus to generated files
- to the TypeScript runtime, which consumes the generated files, and
- to the Rust language server

## Global setup

### node and `pnpm`

Isograph currently uses node version 22, and `pnpm@8.15.3`.

In order to ensure you are using the correct versions of these, you should install `nvm` and run:

```
# in the isograph root folder
nvm use
npm i -g pnpm@8.15.3
```

(There is probably a way to not install `pnpm` globally, and we should figure that out!)

### Rust

I am currently using `rustc 1.81.0 (eeb90cda1 2024-09-04)`. Rust is fairly stable and we don't rely on anything crazy, so it should be fairly safe to keep your `rustc` up-to-date.

You should also install `cargo watch` via `cargo install cargo-watch`.

## Commands related to the compiler and Rust

### Building the compiler

```sh
pnpm watch-rs
```

This will watch and rebuild the compiler for use locally.

### Running the compiler binary directly

The compiler can be run with `./target/debug/isograph_cli`. For example, `pnpm build-pet-demo` runs: `./target/debug/isograph_cli --config ./demos/pet-demo/isograph.config.json`.

If you are using the locally-built compiler from another folder, you should be able to run `$PATH_TO_ISOGRAPH_REPO/target/debug/isograph_cli --config $YOUR_LOCAL_CONFIG`.

### Running the compiler for a specific demo

We also have scripts defined in the `package.json` that make using the compiler easier for the demos. For example:

```sh
# from the root
pnpm build-demos
pnpm watch-pet-demo
pnpm build-pet-demo
pnpm watch-github-demo
pnpm build-github-demo
pnpm watch-isograph-react-demo
pnpm build-isograph-react-demo
```

The `pet-demo` is the most complete, and is probably the one you should use. (See below for more instructions.)

### Running the compiler in a project where `@isograph/compiler` was installed via `yarn`

```sh
yarn run iso
```

### Running Rust tests

```sh
cargo test
```

### Format Rust code

```sh
pnpm format-rust
# or
cargo fmt
# or
pnpm format # which also formats the TypeScript code
```

Many of these tests come from the libraries that we brought in from Relay, and aren't specific to Isograph.

## Commands related to the runtime and JavaScript

### Build the Isograph JavaScript libraries for use in demos

```sh
pnpm -r watch-libs
```

`watch-libs` will watch the source files for changes, and rebuild everything. If you only want to do it once, you can:

```sh
pnpm -r compile
```

### Run unit tests in the libs folder

```sh
pnpm -r test
```

### Format the code

```sh
pnpm format-prettier
# or
pnpm format # which also formats the Rust code
```

### Run the pet demo

```sh
cd demos/pet-demo
pnpm dev
```

You must also run the backend

```sh
cd demos/pet-demo
pnpm backend
```

## How to release a new "main" version of Isograph

Every commit to `main` results in a build, which you can see in [npm](https://www.npmjs.com/package/@isograph/compiler?activeTab=versions). The ones of the form `0.0.0-main-$hash` are generated from a commit to `main`.

## How to release a new "numbered" version of Isograph to npm

- In all package.json files, bump the version number. Don't forget to bump the version number of imports.
- `git add . && git commit -m 'v0.1.0' && git tag v0.1.0 && git push`
- See [this commit releasing 0.2.0](https://github.com/isographlabs/isograph/commit/e36acab1a018e18bdae0558be08952693af3b6a8)
