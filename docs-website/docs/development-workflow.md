# Development workflow

## Useful commands to run in the Isograph repository

### Build the compiler in watch mode

```sh
pnpm watch-rs
```

### Build the Isograph JavaScript libraries for use in demos

```sh
pnpm -r compile
```

This will also typecheck the libs folder.

### Install dependencies, including for the demos

```sh
pnpm -r compile
```

### Run unit tests in the libs folder

```sh
pnpm -r test
```

### Format the code

```sh
pnpm format
```

### Run the demo

```sh
cd demos/pet-demo
pnpm dev
```

### Use the Isograph compiler to generate artifacts for a given demo

```sh
pnpm build-demos
```

See [`package.json`](https://github.com/isographlabs/isograph/blob/main/package.json) for more.

## How to use a local build of the Isograph compiler for a local build

Run `pnpm watch-rs` and pass a relative path to the `isograph_cli` binary:

```
../isograph/target/debug/isograph_cli
```

## How to use a local build of Isograph libraries

Demos are intended for that.

TODO include instructions on how to do this for external libraries.

## How to release a new version of Isograph to npm

- In all package.json files, bump the version number. Don't forget to bump the version number of imports.
- `git add . && git commit -m 'v0.1.0' && git tag v0.1.0 && git push`
- See [this commit releasing 0.2.0](https://github.com/isographlabs/isograph/commit/e36acab1a018e18bdae0558be08952693af3b6a8)
