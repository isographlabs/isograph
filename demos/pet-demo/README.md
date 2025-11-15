# Pet Demo

This is a NextJS app demonstrating the use of Isograph with a locally-running GraphQL API.

## One-time installation

In the root of this repository, run:

```sh
pnpm i
```

You must have `pnpm`, `cargo`, `bacon`, and `rust`. See [the development instructions](https://isograph.dev/docs/development-workflow/).

## Running the app

You should run two commands, in separate terminals, both from the root of the repository:

```sh
pnpm watch-libs
pnpm dev-pet-demo
```

Alternatively, you could run, in separate terminals:

```sh
cd demos/pet-demo && pnpm iso --watch
pnpm watch-libs
pnpm watch-rs
cd demos/pet-demo && pnpm backend
cd demos/pet-demo && pnpm dev
```