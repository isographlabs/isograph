# GitHub Demo

## Prerequisites:

The project depends on `Cargo` and `Rust` for the compiler, as well as `pnpm` to build

* `Rust`: [https://rust-lang.org/tools/install/](https://rust-lang.org/tools/install/)
* `pnpm`: [https://pnpm.io/installation](https://pnpm.io/installation)

## Running locally

In order to run this demo locally:

- create an .env.local file in this folder. It will be ignored by git. It's contents should be:

```sh
NEXT_PUBLIC_GITHUB_TOKEN=$SOME_TOKEN
```

Where `$SOME_TOKEN` is a personal access token. It only needs read access (i.e. the `public_repo` scope). You can create one in [Settings -> Developer Settings -> Personal Access Tokens -> Tokens (Classic)](https://github.com/settings/tokens).

Then, run the following from the root of the repository:

```sh
pnpm i
pnpm -r compile-libs
```

Then, run the project as follows from the `demos/github-demo` folder:

```sh
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the demo in action.

## Modifying the app

You must also run the compiler from the root of the repository:

```sh
cargo build
pnpm run watch-github-demo
```

Changes to the `libs/*` folders must be followed by a `pnpm -r compile`.

Changes to the components in the demo will automatically be picked up by Next, but you will probably have to manually refresh the page.

## Issues

For issues setting up the demo, checkout out [troubleshooting](/docs-website/docs/troubleshooting.md)