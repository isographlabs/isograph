# Pet Demo

This is a NextJS app demonstrating the use of Isograph with a locally-running GraphQL API.

## One-time installation

In the root of this repository, run:

```sh
pnpm i
pnpm -r compile
```

You must have `pnpm` installed.

## Running the app

In order to run the demo, run the following commands in two separate terminals from the `demos/pet-demo` folder:

```sh
npm run backend
```

```sh
pnpm run start
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the demo in action.

## Modifying the app

You must also run the compiler from the root of the repository:

```sh
cargo build
pnpm run watch-pet-demo
```

Changes to the `libs/*` folders must be followed by a `pnpm -r compile`.

Changes to the components in the demo will automatically be picked up by Next, but you will probably have to manually refresh the page.
