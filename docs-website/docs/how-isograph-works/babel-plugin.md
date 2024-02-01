# The babel plugin

## Installation and usage

The babel plugin is installed using

```sh
yarn add --dev @isograph/babel-plugin
```

It is then used via adding the following to your `.babelrc.js`:

```js
{
  "plugins": ["@isograph"]
}
```

## Behavior

The babel plugin will replace calls to `iso` entrypoint with require calls to the appropriate `entrypoint.js` file.

## Requirements

The babel plugin requires an `isograph.config.json` file. It should probably be in the root of your project.
