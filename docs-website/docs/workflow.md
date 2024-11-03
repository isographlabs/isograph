# Workflow for users of Isograph

The workflow for users of Isograph (i.e. developers) is as follows:

- Run the compiler in watch mode via `yarn iso --watch --config ./isograph.config.json`
- Whenever a file containing an `iso` literal changes, or the schema changes, or a schema extension changes, the Isograph compiler will re-compile your code.
- The compiler will write a bunch of files to the folder specified by the `artifact_directory` key in the config, which are used by the Isograph runtime.

## Babel

A babel plugin transforms your `iso` literals. For example, `iso` entrypoint literals are replaced with imports of the associated generated file. ([`swc`](https://github.com/isographlabs/isograph/issues/138) and [`oxc`](https://github.com/isographlabs/isograph/issues/138) plugins are planned — come help out if you're interested!)

## Warnings

- The Isograph compiler panics sometimes. You'll need to fix the issue and restart the compiler in these cases.
- `next` doesn't like certain things, perhaps related to `@component`, so you will sometimes receive a warning overlay with hot module reloading. You can refresh the page to fix this.

## Debugging

- You can use React dev tools. In particular, searching for `@component` will show you where all of your resolvers defined with `@component` are rendered.
- If you call `registerLogFunction` on your environment, you can get additional logs.

## Workflow for modifying Isograph

If you are working on Isograph the project, you may be interested in [this doc](../development-workflow).
