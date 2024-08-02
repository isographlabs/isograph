# Isograph runtime

:::note
This document is intentionally short, because much of the runtime is incomplete and liable to change.
:::

## Initial setup

Currently, there are two things you must to do to use Isograph:

- set up a network function by calling `setNetwork`
- call `subscribe(() => setState({}))`, in order to make some root component re-render whenever anything in the store changes. This is obviously bad for performance.

:::note
You should also see [the quickstart guide](../../quickstart) for more one-time setup, e.g. changes to the `babelrc.js`.
:::

## Big picture

In order to make a network request and read the results, the following occurs:

- the developer calls ``const {fragmentReference} = useLazyReference(iso(`entrypoint Query.HomePage`));``. This will make the network request when that component renders.
  - The babel plugin changes the `iso` entrypoint call to a `require` call that imports the generated `Query/HomePage/entrypoint.ts` file.
- The developer calls `const HomePage = useResult(fragmentReference);`. This will attempt to read the `Query.HomePage` resolver. This may suspend. In particular, if there isn't enough data in the store to read all of the data required by the `HomePage` resolver, the call to `read` will suspend.
  - It is a good practice to pass the `fragmentReference` to a child component, which is wrapped in a `<Suspense>` boundary. This isn't required for `useLazyReference` to work correctly, but it does eliminate some edge cases (namely, if the network response takes too long to come back), and does make refetching on error easier.
  - In the future, there will be other APIs, akin to Relay's `loadQuery` and `useQueryLoader`. These have not been implemented. The `@isograph/react-disposable-state` library contains their building blocks.
- The network response completes, and the normalization AST (part of the `Query/HomePage/entrypoint.ts` file) is used to write the data to the [global store](#store).
- The subscribe callback is triggered, causing the component tree to re-render.
- On second render, the `read(fragmentReference)` call is re-evaluated. This time, there is enough data to read the fields required by `HomePage`, so the `HomePage` resolver function is called. Assuming it is a react component (i.e. the resolver was declared with `@component`), we can then render the component as follows: `<HomePage {...additionaProps} />`.
- When `<HomePage />` renders, it may itself have selected other components (e.g. `Header` or `Avatar`). The data for these was likely provided by initial network request, so they will not suspend, and the whole tree will render.
  - In the future, when Isograph supports `@defer` or `@stream`, child resolvers may suspend at this point. If data in the Isograph store changes, child resolvers may also suspend.

## Store

The Isograph store is a global map from strong IDs or "relative IDs" to fields. It should not be global! But it is, for now.

:::warning
For NextJS, we need to clear the store on every request, since it otherwise would be shared across requests (including for different users).
:::

## Fetching and entrypoints

Declaring an `iso` entrypoint literal results in the creation of an `entrypoint.ts` file. This contains three things:

- The query text (in the future, we will support [persisted queries](https://relay.dev/docs/guides/persisted-queries/) as well.)
- The normalization AST, which is the data structure used to write the network response into the store.
- A hard require of the reader artifact.

In the future, one should be able to generate entrypoints that only contain the query text. The normalization and reader ASTs are not always necessary initially.

## Normalization

When the network response comes back, Isograph iterates the normalization AST and the network response in parallel to write data to the store.

The normalization AST contains information about all of the server fields that are present in the network response, i.e. it does not stop at resolver boundaries. No resolver is present in the normalization AST; it deals purely with server fields.

The network response cannot be written into the store without the help of a normalization AST, because which field is a strong ID field will eventually be configurable, etc. If arbitrary JSON scalars are acceptable parts of the network response (and they currently are, due to being allowed by GraphQL), one also needs a normalization AST to know to treat an arbitrary JSON scalar that looks like a valid "regular ol'" GraphQL response as a scalar.

In addition, when the Relay team adopted normalization ASTs, normalization time fell by 85%, because using a normalization AST means you can avoid introspecting the network response.

### How normalization works

:::warning
This section is especially liable to change.
:::

If an object has a strong ID (for now, this means "if it has an ID field"), the object will be written to the store under that ID. e.g. `{ id: 123, name: "Jerry Garcia" }` will be written to the store as `123: { id: 123, name: "Jerry Garcia" }`.

If an object does not have a strong ID, the object's ID in the Isograph store will be generated based on a path to the nearest parent which has a strong ID. So, if we encounter `{ id: 123, "Jerry Garcia", guitar: { type: "Fender Stratocastor" }}`, this will be written to the store as: `123: { id: 123, name: "Jerry Garcia", guitar: { __link: "123.guitar" } }` and `123.guitar: { type: "Fender Stratocastor" }`.

## Reading

:::warning
This section is especially liable to change.
:::

Right now, when a resolver is read, all server fields selected by that resolver are read. If the resolver selected any child resolvers (e.g. if `full_name` is a resolver in `User.address { full_name }`), then those are also read.

If a resolver is **not** defined with `@component`, then if any selected server fields are missing or if any selected child resolvers return `{ kind: "MissingData" }`, then the resolver itself returns `{ kind: "MissingData" }`.

On the other hand, resolvers defined with `@component` will never return `{ kind: "MissingData" }` when read out by parent resolvers. Instead, they only suspend when rendered. This allows you to strategically place suspense boundaries.

## Changes to data in the store and `subscribe`

If any data changes in the store (for now, this can only occur through other network responses being received and normalized), the top-level `subscribe` callback is called. Thus, the entire component tree re-renders.

In the future, reader ASTs can be used to isolate re-renders to just parts of the component tree that need to re-render.
