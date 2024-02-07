# Quickstart guide

In this quickstart guide, we will add Isograph to an existing NextJS project. We will use the free and publicly available [Star Wars GraphQL API](https://studio.apollographql.com/public/star-wars-swapi/variant/current/home).

You can view the end result of following this quickstart guide in [this repository](https://github.com/isographlabs/quickstart).

:::note
This is the process for adding Isograph to an existing **NextJS project**. However, it shouldn't be that different to add it to a project in another framework.

If you don't have a NextJS project handy, run `npx create-next-app@latest` and proceed with this quickstart! (In this example, we're using the `src` directory and not using the App Router.)

This currently requires NextJS to be run with Babel and for React not to be run in strict mode.
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
  presets: ['next/babel'],
  plugins: ['@isograph'],
};
```

And add an alias to your `tsconfig.json`. The alias should point to wherever your `artifact_directory` is located, followed by `__isograph/*`. So, if our `artifact_directory` is `./src`, we would define the alias to be `./src/__isograph/*`. (See the `isograph.config.json` step.)

```json
"paths": {
  "@iso/*": ["./src/__isograph/*"]
},
```

## Disable React strict mode

NextJS defaults to using strict mode. For Isograph to work, you must disable strict mode in your `next.config.js` file:

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

In our case, we can do that by change our `src/pages/_app.tsx` file to look like:

```tsx
import { useMemo } from 'react';
import type { AppProps } from 'next/app';
import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch(
    'https://swapi-graphql.netlify.app/.netlify/functions/index',
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query: queryText, variables }),
    },
  ).then((response) => response.json());
  return promise;
}

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(
    () => createIsographEnvironment(createIsographStore(), makeNetworkRequest),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Component {...pageProps} />
    </IsographEnvironmentProvider>
  );
}
```

We're also creating the Isograph store in this step, which is the in-memory key-value store where Isograph keeps the network data you have received.

:::warning
With NextJS, it is **extremely important** to not create the environment at the top level (i.e. in module scope.) If you do this, **NextJS will reuse the environment across requests,** so different users will share the same environment!

Create the environment during the render of a component is sufficient to avoid this. However, you should also memoize the creation of the environment so that if (for whatever reason), your `App` component re-renders, you do not recreate the environment, thus losing data.
:::

:::note
You may need to provide a bearer token if you are using a public API, such as the GitHub API. See [this GitHub demo](https://github.com/rbalicki2/github-isograph-demo/tree/885530d74d9b8fb374dfe7d0ebdab7185d207c3a/src/isograph-components/SetNetworkWrapper.tsx) for an example of how to do with a token that you receive from OAuth. See also the `[...nextauth].tsx` file in the same repo.
:::

## Create an Episode List component

**Finally**, we can get to writing some Isograph components. Let's define the Isograph client field that "is" your app! Create a file in `src/components/EpisodeList.tsx` containing the following:

```tsx
import React from 'react';
import { iso } from '@iso';

// Note: normally, the "root" field is called Query, but in the Star Wars
// GraphQL schema it is called Root. Odd!
export const EpisodeList = iso(`
  field Root.EpisodeList @component {
    allFilms {
      films {
        id,
        title,
        episodeID,
      },
    },
  }
`)(function EpisodeListComponent({ data }) {
  const filmsSorted = data.allFilms?.films ?? [];
  filmsSorted.sort((film1, film2) => {
    if (film1?.episodeID == null || film2?.episodeID == null) {
      throw new Error(
        'This API does not return null films or null episode IDs.',
      );
    }
    return film1.episodeID > film2.episodeID ? 1 : -1;
  });

  return (
    <>
      <h1>Star Wars film archive</h1>
      {filmsSorted.map((film) => (
        <React.Fragment key={film?.id}>
          <h2>
            Episode {film?.episodeID}: {film?.title}
          </h2>
        </React.Fragment>
      ))}
    </>
  );
});
```

## Fetch that Episode List

That Isograph component isn't doing much on its own. We need to provide a way to fetch its data and render the results. So, create a file at `src/components/EpisodeListRoute.tsx`, and make its contents:

```tsx
import React from 'react';
import { iso, useLazyReference, useRead } from '@isograph/react';
import EpisodeListEntrypoint from '@iso/Root/EpisodeList/entrypoint';

export default function EpisodeListRoute() {
  return (
    <React.Suspense fallback={'Data is loading...'}>
      <Inner />
    </React.Suspense>
  );
}

function Inner() {
  const { queryReference } = useLazyReference(
    iso(`entrypoint Root.EpisodeList`),
    {
      /* query variables */
    },
  );

  const Component = useRead(queryReference);
  const additionalRenderProps = {};
  return <Component {...additionalRenderProps} />;
}
```

and change `src/pages/index.tsx` to be:

```tsx
import EpisodeListRoute from '@/components/EpisodeListRoute';

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

A key principle of React is that you can divide your components into subcomponents. Let's do that! For example, create `src/components/CharacterSummary.tsx` containing:

```tsx
import React from 'react';
import { iso } from '@iso';

export const CharacterSummary = iso(`
  field Person.CharacterSummary @component {
    name,
    homeworld {
      name,
    },
  }
`)(function CharacterSummaryComponent({ data }) {
  return (
    <li>
      {data.name}, from the planet {data.homeworld?.name}
    </li>
  );
});
```

You might use this component by modifying `EpisodeList.tsx` to be the following. Note the two new sections:

```tsx
import React from 'react';
import { iso } from '@iso';

// Note: normally, the "root" field is called Query, but in the Star Wars
// GraphQL schema it is called Root. Odd!
export const EpisodeList = iso(`
  field Root.EpisodeList @component {
    allFilms {
      films {
        id,
        title,
        episodeID,

        # THIS IS NEW
        characterConnection {
          characters {
            id,
            CharacterSummary,
          },
        },
      },
    },
  }
`)(function EpisodeListComponent({ data }) {
  const filmsSorted = data.allFilms?.films ?? [];
  filmsSorted.sort((film1, film2) => {
    if (film1?.episodeID == null || film2?.episodeID == null) {
      throw new Error(
        'This API does not return null films or null episode IDs.',
      );
    }
    return film1.episodeID > film2.episodeID ? 1 : -1;
  });

  return (
    <>
      <h1>Star Wars film archive</h1>
      {filmsSorted.map((film) => (
        <React.Fragment key={film?.id}>
          <h2>
            Episode {film?.episodeID}: {film?.title}
          </h2>
          {/*
           THE FOLLOWING IS NEW
          */}
          <div style={{ marginLeft: 20 }}>
            Featuring
            <ul>
              {film?.characterConnection?.characters?.map((character) => {
                return <character.CharacterSummary key={character.id} />;
              })}
            </ul>
          </div>
        </React.Fragment>
      ))}
    </>
  );
});
```

Now, if you refresh, you'll see a list of Star Wars characters that show up in each movie! Wow!

## Congratulations

Congratulations! You just built your first Isograph app.
