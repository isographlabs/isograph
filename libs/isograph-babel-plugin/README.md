# `@isograph/babel-plugin`

This package exposes a babel plugin for use with Isograph. It is highly recommended.

## Installation:

First, install the package:

```bash
yarn add @isograph/babel-plugin
```

Next, add it to your `.babelrc`:

```js
module.exports = {
  plugins: ['@isograph'],
};
```

## What does it do?

This package changes calls to `iso` entrypoint to imports of the generated artifact. For example, `` iso`entrypoint Query.HomePage` `` might get replaced with an import of `"../__isograph/Query/HomePage/entrypoint.ts"`. In a file Babel parses as a script, it becomes `require("../__isograph/Query/HomePage/entrypoint.ts").default` instead.

A `field` or `pointer` literal called with a resolver becomes that resolver. An `entity` literal, such as ``iso(`entity Owner`)``, becomes `void 0`, since it declares an entity and has no runtime value.

The artifact is under `output_directory` from the config, relative to the config file. Without `output_directory`, it is under the config file's directory.

## Requirements

For this babel plugin to work, it must find an `isograph.config.json` file. It is safe to put one at the root of your project.

:::warning
`yarn iso --config $PATH` will work if the config is not named `isograph.config.json`, or is not found in the root of the project. But the babel plugin will not (yet!)
:::
