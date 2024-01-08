---
sidebar_position: 2
---

# Quickstart guide

:::info
This quickstart guide is somewhat incomplete. If there is a missing step, let me know! Your best bets are to emulate the demo projects.
:::

## Adding Isograph to an existing NextJS project

The process for adding Isograph to an existing NextJS project is described in this document. It shouldn't be that different to add it to a project in another framework.

### Install the compiler, babel plugin and runtime

```sh
yarn install --dev @isograph/compiler@main
yarn install --dev @isograph/babel-plugin@main
yarn install @isograph/react@main
```

:::info
For now, you must install the `@main` versions of the packages.
:::

Installing the compiler also adds the command `yarn iso`.

### Install the babel plugin and add a recommended alias

Install the babel plugin in your `.babelrc.js`:

```js
module.exports = {
  presets: ["next/babel"],
  plugins: ["@isograph"],
};
```

And add an alias to your `tsconfig.json`. The alias should point to wherever your `artifact_directory` is located. (See the `isograph.config.json` step.)

```json
"paths": {
  "@iso/*": ["./src/__isograph/*"]
},
```

### Disable React strict mode

Isograph is currently incompatible with React strict mode. Being compatible with strict mode means that (during dev), we will necessarily refetch every query twice. I will eventually lift this restriction, but for now, disable strict mode.

```js
// next.config.js
const nextConfig = {
  reactStrictMode: false,
};
```

### Create an `isograph.config.json` file

Create an `isograph.config.json` file. Example contents:

```json
{
  "project_root": "./src/components",
  "artifact_directory": "./src",
  "schema": "./backend/schema.graphql"
}
```

:::note
Note that (for now!) the `artifact_directory` should not be within the `project_root`, as this causes an infinite build-rebuild loop. This is fixable.
:::

:::note
Isograph generates relative paths, so it doesn't really matter where you put your `artifact_directory`.
:::

You should also have your graphql schema available at the point. An example schema might be:

```graphql
type Query {
  viewer: Viewer
}

type Viewer {
  name: String
}
```

The `schema` field should point to this file.

### Run the isograph compiler in watch mode

```sh
yarn iso --config ./isograph.config.json --watch
```

The compiler will start running, but since we haven't written any isograph literals, it won't do much.

:::note
Isograph **will** generate a `__refetch` artifact for each type that has an `id: ID!` field.
:::

### Teach isograph about your backend

If your backend is running at `localhost:4000/graphql`, you might put the following code somewhere where it will execute. For example, you might place it at `pages/index.tsx` in a NextJS app:

```tsx
import { setNetwork } from "@isograph/react";
function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch("http://localhost:4000/graphql", {
    method: "POST",
    headers: {
      // You may need to include a bearer token, for example if you are hitting
      // the GitHub API.
      // "Authorization": "Bearer " + BEARER_TOKEN,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
  return promise;
}
setNetwork(makeNetworkRequest);
```

:::note
You may need to provide a bearer token if you are using a public API, like that of GitHub. See [this GitHub demo](https://github.com/rbalicki2/github-isograph-demo/tree/885530d74d9b8fb374dfe7d0ebdab7185d207c3a/src/isograph-components/SetNetworkWrapper.tsx) for an example of how to do with a token that you receive from OAuth. See also the `[...nextauth].tsx` file in the same repo.
:::

### Tell Isograph to re-render whenever new data is received

Right now, there is no granular re-rendering in Isograph. (A lot of features are missing!) Instead, we just add a hook at the root that re-renders the entire tree if anything changes in the Isograph store. This can go in the `App` component in `pages/index.tsx`, defined in a future step.

```tsx
// N.B. we are rerendering the root component on any store change
// here. Isograph will support more fine-grained re-rendering in
// the future, and this will be done automatically as part of
// useLazyReference.
const [, setState] = useState<object | void>();
useEffect(() => {
  return subscribe(() => setState({}));
}, []);
```

### Create isograph literals

**Finally**, we can get to writing some Isograph components. Let's define the Isograph resolver that "is" your home route component! You might create the following in `src/home_route.tsx`:

```tsx
import React from "react";
import { iso } from "@isograph/react";
import { ResolverParameterType as HomeRouteParams } from "@iso/Query/home_route/reader.isograph";

// You must export the iso literal
export const home_route = iso<HomeRouteParams, ReturnType<typeof HomeRoute>>`
  Query.home_route @component {
    viewer {
      name,
    },
  }
`(HomeRoute);

function HomeRoute(props: HomeRouteParams) {
  return <h1>Hello {props.data.viewer?.name}</h1>;
}
```

:::info
Note: for now, we have a lot of types to provide. These are fairly boilerplate, and should be able to removed in the future. For now, provide them!
:::

### Fetch that Isograph component:

That Isograph component isn't doing much on its own. We need to provide a way to fetch its data and render the results. So, in `pages/index.tsx`, add:

```tsx
import React from "react";
import { subscribe, isoFetch, useLazyReference, read } from "@isograph/react";

export default function App() {
  // The "subscribe" code block can go here

  return (
    <React.Suspense fallback={"suspending"}>
      <Inner />
    </React.Suspense>
  );
}

function Inner() {
  const { queryReference } = useLazyReference(
    isoFetch<typeof HomeRouteEntrypoint>`
      Query.home_route
    `,
    {
      /* query variables */
    }
  );

  return read(queryReference)({
    /* additional runtime props */
  });
}
```

Now, if you navigate to your home screen, you should see "Hello" and your name! (You actually won't, because there is no GraphQL server running on port 4000. But, a future version of this quickstart will hit an API, like the Star Wars API, that is publicly available and free.)
