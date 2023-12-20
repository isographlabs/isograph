---
sidebar_position: 2
---

# Quickstart guide

## Adding Isograph to an existing compiler

You can add Isograph to an existing project (for example, a NextJS app), as follows:

### Install the compiler and runtime

```sh
# Note: you cannot (yet) install @isograph/compiler. That will install
# the wrong version.
yarn install --dev @isograph/compiler@0.0.0-main-c23726d7
yarn install @isograph/react@0.0.0-main-c23726d7
```

Installing the compiler also adds the command `yarn iso`.

### Create an `isograph.config.json` file

Example contents:

```json
{
  "project_root": "./src/components",
  "artifact_directory": "./src",
  "schema": "./backend/schema.graphql",
  "schema_extensions": []
}
```

Note that (for now!) the `artifact_directory` should not be within the `project_root`. Isograph generates relative paths, so it doesn't really matter where you put your `artifact_directory`.

You should also have your graphql schema available at the point. An example schema might be:

```graphql
type Query {
  viewer: Viewer
}

type Viewer {
  name: String
}
```

### Run the isograph compiler in watch mode

```sh
yarn iso --config ./isograph.config.json --watch
```

The compiler will start running, but since we haven't written any isograph literals, it won't do much.

### Teach isograph about your backend

If your backend is running at `localhost:4000/graphql`, you might put the following code somewhere where it will execute. For example, you might place it at `pages/index.tsx` in a NextJS app:

```tsx
import { setNetwork } from "@isograph/react";
function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch("http://localhost:4000/graphql", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
  return promise;
}
setNetwork(makeNetworkRequest);
```

### Create isograph literals

Next, let's define the Isograph resolver that "is" your home route component! You might create the following in `src/home_route.tsx`:

```tsx
import React from "react";
import { iso } from "@isograph/react";
import { ResolverParameterType as HomeRouteParams } from "@iso/Query/home_route/reader.isograph";

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

### Fetch that Isograph component:

That Isograph component isn't doing much on its own. We need to provide a way to fetch its data and render the results. So, in `pages/index.tsx`, add:

```tsx
import React from "react";
import { subscribe, isoFetch, useLazyReference, read } from "@isograph/react";
import NoSSR from "react-no-ssr";

export default function App() {
  // N.B. we are rerendering the root component on any store change
  // here. Isograph will support more fine-grained re-rendering in
  // the future, and this will be done automatically as part of
  // useLazyReference.
  const [, setState] = React.useState<object | void>();
  React.useEffect(() => {
    return subscribe(() => setState({}));
  });

  return (
    <NoSSR>
      <React.Suspense fallback={"suspending"}>
        <Inner />
      </React.Suspense>
    </NoSSR>
  );
}

function Inner() {
  const { queryReference } = useLazyReference(
    isoFetch<typeof HomeRouteEntrypoint>`
      Query.home_route
    `,
    {}
  );

  return read(queryReference)({
    /* additional props */
  });
}
```

Now, if you navigate to your home screen, you should see "Hello " and your name!
