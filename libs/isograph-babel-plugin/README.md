# `@isograph/babel-plugin`

This package exposes a babel plugin for use with Isograph. It is highly recommended.

## Installation:

First, install the package:

```bash
yarn add @isograph/babel-plugin@0.0.0-main-b5263898
```

Next, add it to your `.babelrc`:

```js
module.exports = {
  plugins: ['@isograph'],
};
```

## What does it do?

This package changes calls to `iso` entrypoint to `require` calls for the generated artifact. For example, `` iso`entrypoint Query.HomePage` `` might get replaced with `require("../__isograph/Query/HomePage/entrypoint.ts")`.

## Requirements

For this babel plugin to work, it must find an `isograph.config.json` file. It is safe to put one at the root of your project.

:::warning
`yarn iso --config $PATH` will work if the config is not named `isograph.config.json`, or is not found in the root of the project. But the babel plugin will not (yet!)
:::

## What about SWC?

The backlog includes developing an SWC plugin. It has not been done, yet.
