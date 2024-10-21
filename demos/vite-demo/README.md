# React + TypeScript + Vite + Isograph

This template provides a minimal setup to get Isograph working in Vite with HMR. It queries the [GraphQL Pokemon API](https://graphql-pokemon.js.org) to display the original 151 Pokemon using Isograph components.

## Getting started

Install dependencies:

```bash
pnpm install
```

From the root of the project, start the Isograph compiler:

```bash
pnpm run watch-vite-demo
```

From the `demos/vite-demo`, start the Vite server:

```bash
pnpm run dev
```

## How to Configure

The Vite configuration is slightly different from the NextJS configuration found on the [Quickstart](https://isograph.dev/docs/quickstart/) guide.

Review the following files to see the proper configuration you'll need to match to get a Vite project working with Isograph after running the Vite [Getting Started](https://vite.dev/guide/#scaffolding-your-first-vite-project) steps:

1. `vite.config.ts`
2. `.babelrc.json`
3. `tsconfig.app.json`
4. `tsconfig.node.json`
