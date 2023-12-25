# `@isograph/compiler`

This package installs the Isograph compiler under the `yarn iso` command.

## Usage

```bash
yarn iso --config ./isograph.config.json
# or
yarn iso --config ./isograph.config.json --watch
```

## Requirements

This requires a valid Isograph config. See [the Isograph config docs](../../docs-website/docs/isograph-config.md).

:::warning
`yarn iso --config $PATH` will work if the config is not named `isograph.config.json`, or is not found in the root of the project. But the babel plugin will not (yet!)
:::

## Options

- `--config` this is required, and is a relative path to the Isograph config.
- `--watch` if passed, this starts the compiler in watch mode.
