# The babel plugin

## Installation and usage

The babel plugin is installed using

```sh
yarn add --dev @isograph/babel-plugin
```

It is then used via adding the following to your `.babelrc.js`:

```json
{
  "plugins": ["@isograph"]
}
```

## Behavior

The babel plugin will replace calls to `iso` entrypoint with require calls to the appropriate `entrypoint.js` file.

It will transform `iso` field definitions as follows:

```ts
// your source contains
export const foo = iso(`field Query.Foo { whatever }`)(({ data }) => {
  doStuff();
});
```

```ts
// and this is transformed into
export const foo = ({ data }) => doStuff();
```

## Requirements

The babel plugin requires an `isograph.config.json` file. It should probably be in the root of your project.
