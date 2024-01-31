# Quickstart guide

In this quickstart guide, we will add Isograph to an existing NextJS project. We will use the free and publicly available [Star Wars GraphQL API](https://studio.apollographql.com/public/star-wars-swapi/variant/current/home).

:::note
This is the process for adding Isograph to an existing **NextJS project**. However, it shouldn't be that different to add it to a project in another framework.

If you don't have a NextJS project handy, you can follow the instructions [here](https://nextjs.org/docs/getting-started/installation) and proceed with this quickstart!

This currently requires NextJS to be run with Babel and for React to be run in React strict mode.
:::

## Install the compiler, babel plugin and runtime

```sh
yarn add --dev @isograph/compiler@main
yarn add --dev @isograph/babel-plugin@main
yarn add @isograph/react@main
```

:::info
For now, you must install the `@main` versions of the packages.
:::

Installing the compiler also adds the command `yarn iso` and `yarn iso --watch`. But hang tight — before this command works, you'll need to set up some folders, download your schema and create an `isograph.config.json` file!

## Install the babel plugin and add a recommended alias

Install the babel plugin in your `.babelrc.js`. If this file does not exist (and it is not usually created in a new NextJS project), you can use the following:

```js
module.exports = {
  presets: ["next/babel"],
  plugins: ["@isograph"],
};
```

And add an alias to your `tsconfig.json`. The alias should point to wherever your `artifact_directory` is located, followed by `__isograph/*`. So, if our `artifact_directory` is `./src`, we would define the alias to be `./src/__isograph/*`. (See the `isograph.config.json` step.)

```json
"paths": {
  "@iso/*": ["./src/__isograph/*"]
},
```

## Disable React strict mode

Isograph is currently incompatible with React strict mode. Being compatible with strict mode means that (during dev), queries must be fetched twice. This restriction will eventually be listed. For now, disable strict mode.

```js
// next.config.js
const nextConfig = {
  reactStrictMode: false,
};
```

## Create an `isograph.config.json` file

Create an `isograph.config.json` file. Example contents:

```json
{
  "project_root": "./src/components",
  "artifact_directory": "./src",
  "schema": "./schema.graphql"
}
```

Then, create the `project_root` directory, e.g.:

```sh
mkdir -p src/components
```

:::note
Note that (for now!) the `artifact_directory` should not be within the `project_root`, as this causes an infinite build-rebuild loop. This is fixable.
:::

:::note
Isograph generates relative paths, so it doesn't really matter where you put your `artifact_directory`, as long as it isn't within your `project_root`.
:::

## Download the schema

Download your GraphQL schema and put it in `./schema.graphql`:

```sh
curl https://raw.githubusercontent.com/graphql/swapi-graphql/master/schema.graphql > ./schema.graphql
```

## Run the isograph compiler in watch mode

```sh
yarn iso --watch
```

The compiler will start running, but since we haven't written any isograph literals, it won't do much.

:::note
Isograph **will** generate a `__refetch` artifact for each type that has an `id: ID!` field.
:::

## Teach isograph about your GraphQL server

Isograph requires some initial setup to teach it how to make API calls to your GraphQL server. The GraphQL server we will hit is running at `https://swapi-graphql.netlify.app/.netlify/functions/index`.

In our case, we can do that by adding the following to the top of our `src/pages/_app.tsx`:

```tsx
import { setNetwork } from "@isograph/react";
function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch(
    "https://swapi-graphql.netlify.app/.netlify/functions/index",
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query: queryText, variables }),
    }
  ).then((response) => response.json());
  return promise;
}
setNetwork(makeNetworkRequest);
```

:::note
You may need to provide a bearer token if you are using a public API, like that of GitHub. See [this GitHub demo](https://github.com/rbalicki2/github-isograph-demo/tree/885530d74d9b8fb374dfe7d0ebdab7185d207c3a/src/isograph-components/SetNetworkWrapper.tsx) for an example of how to do with a token that you receive from OAuth. See also the `[...nextauth].tsx` file in the same repo.
:::

## Tell Isograph to re-render whenever new data is received

Right now, there is no granular re-rendering in Isograph. (A lot of features are missing!) Instead, we add a hook at the root that re-renders the entire tree if anything changes in the Isograph store. This can go in the `App` component that is in `src/pages/_app.tsx`.

```tsx
import { useState } from "React";

// ... setNetworkRequest code from the previous step goes here ...

import "@/styles/globals.css";
import type { AppProps } from "next/app";

export default function App({ Component, pageProps }: AppProps) {
  // N.B. we are rerendering the root component on any store change
  // here. Isograph will support more fine-grained re-rendering in
  // the future, and this will be done automatically as part of
  // useLazyReference.
  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(() => setState({}));
  }, []);

  return <Component {...pageProps} />;
}
```

## Tell NextJS not to re-use values in the store when you refresh

Add the following to each page (e.g. to `pages/index.tsx`). Otherwise, NextJS will reuse the value in the store for all network requests, which is a serious privacy liability.

```tsx
import { clearStore } from "@isograph/react";
export async function getServerSideProps() {
  clearStore();
  return { props: {} };
}
```

## Create isograph literals

**Finally**, we can get to writing some Isograph components. Let's define the Isograph resolver that "is" your home route component! Create a file in `src/components/EpisodeList.tsx` containing the following:

```tsx
import React from "react";
import { iso } from "@isograph/react";
import { ResolverParameterType as EpisodeListParams } from "@iso/Root/EpisodeList/reader.isograph";

export const EpisodeList = iso<EpisodeListParams>`
  # Note: normally, the "root" field is called Query, but in the Star Wars API
  # it is called Root. Odd!
  Root.EpisodeList @component {
    allFilms {
      films {
        id,
        title,
        episodeID,
      },
    },
  }
`(EpisodeListComponent);

function EpisodeListComponent({ data }: EpisodeListParams) {
  const filmsSorted = data.allFilms?.films ?? [];
  filmsSorted.sort((film1, film2) =>
    film1?.episodeID > film2?.episodeID ? 1 : -1
  );

  return (
    <ul>
      {filmsSorted.map((film) => (
        <li>
          Episode {film.episodeID}: {film.title}
        </li>
      ))}
    </ul>
  );
}
```

:::note
That's a lot of types! Soon, it won't be necessary to provide them.
:::

## Fetch that Isograph component:

That Isograph component isn't doing much on its own. We need to provide a way to fetch its data and render the results. So, create a file at `src/components/EpisodeListRoute.tsx`, and make its contents:

```tsx
import React from "react";
import { iso, useLazyReference, read } from "@isograph/react";
import EpisodeListEntrypoint from "@iso/Root/EpisodeList/entrypoint.isograph";

export default function EpisodeListRoute() {
  return (
    <React.Suspense fallback={"Data is loading..."}>
      <Inner />
    </React.Suspense>
  );
}

function Inner() {
  const { queryReference } = useLazyReference<typeof EpisodeListEntrypoint>(
    iso`entrypoint Root.EpisodeList`,
    {
      /* query variables */
    }
  );

  const Component = useRead(queryReference);
  const additionalRenderProps = {};
  return <Component {...additionalRenderProps} />;
}
```

and change `pages/index.tsx` to be:

```tsx
import EpisodeListRoute from "@/components/EpisodeListRoute";

export default function Home() {
  return <EpisodeListRoute />;
}
```

## Run the NextJS app

Start the NextJS app with

```
yarn run dev
```

Now, open your browser to `localhost:3000` and see the list of Star Wars episodes! Congratulations!

In the network tab, you'll see a network request to `https://swapi-graphql.netlify.app/.netlify/functions/index`, the response of which contains a list of Star Wars movies!

## Add a subcomponent

Next, you might create another Isograph component. For example, if you create a file `src/components/person_component.tsx` as:

```tsx
import React from "react";
import { iso } from "@isograph/react";
import { ResolverParameterType as CharacterSummaryParams } from "@iso/Person/CharacterSummary/reader.isograph";

export const CharacterSummary = iso<CharacterSummaryParams>`
  Person.CharacterSummary @component {
    name,
    homeworld {
      name,
    },
  }
`(CharacterSummaryComponent);

function PersonComponentComponent({ data }: CharacterSummaryParams) {
  return (
    <>
      {data.name}, from the planet {data.homeworld?.name}
    </>
  );
}
```

You might use this component by modifying `EpisodeList.tsx` to be the following. Note the two new sections:

```tsx
import React from "react";
import { iso } from "@isograph/react";
import { ResolverParameterType as EpisodeListParams } from "@iso/Root/EpisodeList/reader.isograph";

export const EpisodeList = iso<EpisodeListParams>`
  # Note: normally, the "root" field is called Query, but in the Star Wars API
  # it is called Root. Odd!
  Root.EpisodeList @component {
    allFilms {
      films {
        id,
        title,
        episodeID,

        # THIS IS NEW
        characterConnection {
          characters {
            CharacterSummary,
          },
        },
      },
    },
  }
`(EpisodeListComponent);

function EpisodeListComponent({ data }: EpisodeListParams) {
  const filmsSorted = data.allFilms?.films ?? [];
  filmsSorted.sort((film1, film2) =>
    film1?.episodeID > film2?.episodeID ? 1 : -1
  );

  return (
    <ul>
      {filmsSorted.map((film) => (
        <li>
          Episode {film.episodeID}: {film.title}
          {/*
           THE FOLLOWING IS NEW
          */}
          <div style={{ marginLeft: 20 }}>
            Featuring
            {film?.characterConnection?.characters?.map((character) => {
              return <character.CharacterSummary />;
            })}
          </div>
        </li>
      ))}
    </ul>
  );
}
```

Now, if you refresh, you'll see a list of Star Wars characters that show up in each movie! Wow!

## Congratulations

Congratulations! You just built your first Isograph app.
