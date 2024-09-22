import DataTypeSrc from './assets/data-type.png';

# Quickstart guide

In this quickstart guide, we will create a new NextJS project and add Isograph to it. Then we'll use the free and publicly available [Star Wars GraphQL API](https://studio.apollographql.com/public/star-wars-swapi/variant/current/home).

You can view the end result of following this quickstart guide in [this repository](https://github.com/isographlabs/quickstart).

## Install NextJS

In a newly-created empty directory, run:

```sh
npx create-next-app@latest . \
  --ts --eslint --no-app --src-dir \
  --no-tailwind --import-alias "@/*"
```

This will install a NextJS app in this folder. Run it with `npm run dev`.

## Install the compiler, Babel plugin and runtime

```sh
yarn add --dev @isograph/compiler
yarn add --dev @isograph/babel-plugin
yarn add @isograph/react
```

Installing the compiler also adds the command `yarn iso` and `yarn iso --watch`. But hang tight — before this command works, you'll need to create a folder, download your schema and create an `isograph.config.json` file!

## Create an `isograph.config.json`

Create an `isograph.config.json` file. You can use the following for this quickstart:

```json
{
  "project_root": "./src/components",
  "artifact_directory": "./src/components",
  "schema": "./schema.graphql"
}
```

:::note
The `artifact_directory` field is optional, and defaults to the `project_root`. Feel free to skip it.
:::

## Add aliases to `tsconfig.json`

Add two aliases to your `tsconfig.json`'s `compilerOptions` field. These alias should point to `artifact_directory`, followed by `__isograph/*` and `__isograph/iso.ts`. Here is a snippet of a `tsconfig` showing the `paths` field, correctly set up for this quickstart:

```json
{
  "compilerOptions": {
    "paths": {
      "@iso/*": ["./src/components/__isograph/*"],
      "@iso": ["./src/components/__isograph/iso.ts"]
    }
  }
}
```

:::note
We won't be using the first alias in this demo, but it is a best practice for Isograph projects.
:::

## Disable React strict mode

NextJS defaults to using strict mode. Isograph is currently incompatible with strict mode. Disable strict mode in your `next.config.js` file as follows:

```js
// next.config.js
const nextConfig = {
  reactStrictMode: false,
};
```

:::note Why is this necessary?
See [this FAQ item](/docs/faq/#why-does-isograph-not-support-strict-mode/) for an explanation.
:::

## Create a `.babelrc.js`

To enable Babel and the Isograph Babel plugin, create a `.babelrc.js` with the following contents:

```js
module.exports = {
  presets: ['next/babel'],
  plugins: ['@isograph'],
};
```

:::note What about SWC?
Isograph currently requires a Babel plugin, but there is an [open, good first issue](https://github.com/isographlabs/isograph/issues/13) to make it work with SWC.
:::

## Download the schema

Download your GraphQL schema and put it in `./schema.graphql`:

```sh
curl https://raw.githubusercontent.com/graphql/swapi-graphql/master/schema.graphql > ./schema.graphql
```

## Run the compiler in watch mode

```sh
yarn iso --watch
```

The compiler will start running, but since we haven't written any Isograph literals, it won't do much.

:::note
The Isograph compiler can be a bit finicky, especially if you're still learning the syntax. If the process stops, don't panic — just fix the error and restart the compiler.
:::

## Teach Isograph how to make network requests

Isograph requires some initial setup to teach it how to make API calls to your GraphQL server. The GraphQL server we will hit is running at `https://swapi-graphql.netlify.app/.netlify/functions/index`.

In our case, we can do that by changing our `src/pages/_app.tsx` file to look like:

```tsx
import { useMemo, Suspense } from 'react';
import type { AppProps } from 'next/app';
import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  const promise = fetch(
    'https://swapi-graphql.netlify.app/.netlify/functions/index',
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query: queryText, variables }),
    },
  ).then(async (response) => {
    const json = await response.json();

    if (response.ok) {
      return json;
    } else {
      throw new Error('NetworkError', {
        cause: json,
      });
    }
  });
  return promise;
}

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(
    () => createIsographEnvironment(createIsographStore(), makeNetworkRequest),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Suspense fallback="loading">
        <Component {...pageProps} />
      </Suspense>
    </IsographEnvironmentProvider>
  );
}
```

In this step, we created some context that holds the Isograph environment. The Isograph environment contains the data that we have received from the network and knows how to make network requests to the GraphQL backend.

:::note Why are we wrapping `Component` in a suspense boundary?
We're wrapping the inner `<Component />` in a suspense boundary because later, we'll render `<HomePage />`. This component will suspend if data is missing. When the network request completes and the component unsuspends, React will re-render the children of the suspense boundary.

When React re-renders the children of a suspense boundary, their hooks lose all state. We memoize the Isograph environment out side of the suspense boundary because, if the environment were defined within the suspense boundary, it would be recreated in an infinite loop :'(
:::

## Create the `Root.HomePage` component

**Finally**, we can get to building our first client field, the `Root.HomePage` component!

An Isograph app will be almost entirely made up of client fields. There are two important important facts about client fields that you should know:

- they can reference each other. In this quickstart, `Root.HomePage` will reference `Film.FilmSummary`.
- they can return arbitrary values. In this quickstart, both fields will return React elements. A field that return a React elements is called a _client component field_.

So, let's define our first field, `Root.HomePage`, by making sure that `yarn iso --watch` is running and then creating a file (e.g. `src/components/HomePage.tsx`) containing the following:

```tsx
import React from 'react';
import { iso } from '@iso';

export const HomePage = iso(`
  field Root.HomePage @component {}
`)(function HomePageComponent(data) {
  return 'Hello from the home page!';
});
```

That's it! That's our first Isograph component. Let's break down what we just did.

- We defined a field named `HomePage` on the [type `Root`](https://github.com/isographlabs/quickstart/blob/master/schema.graphql#L643-L662), which our GraphQL schema has defined as our query ["root operation type"](https://github.com/isographlabs/quickstart/blob/master/schema.graphql#L2).
- We wrote `@component` to tell the Isograph compiler that this field is a component.
- Then, we passed a simple React component to this `iso` literal.

Let's proceed by **selecting some fields**. Modify the export as follows:

```tsx
export const HomePage = iso(`
  field Root.HomePage @component {
    allFilms {
      films {
        id
        title
        episodeID
      }
    }
  }
`)(function HomePageComponent(data) {
  return 'Hello from the home page!';
});
```

Now, when the component is called, the first argument (`data`) will have type:

```tsx
type Data = {
  allFilms: {
    films: ({
      id: string;
      title: string | null;
      episodeID: number | null;
    } | null)[];
  } | null;
};
```

Every time you save, the Isograph compiler will recompile everything, including re-generating the type of the iso function. This means that TypeScript knows the type of the `data` parameter without you having to do anything!

<img src={DataTypeSrc} height="308" />

Let's complete this component by returning a list of the films, their titles and episode names. The entire file should now look like:

```tsx
import React, { useMemo } from 'react';
import { iso } from '@iso';

function nonNullable<T>(value: T): value is NonNullable<T> {
  return value != null;
}

function toSorted<T>(arr: T[], comparator: (a: T, b: T) => number): T[] {
  const sorted = [...arr];
  sorted.sort(comparator);
  return sorted;
}

export const HomePage = iso(`
  field Root.HomePage @component {
    allFilms {
      films {
        id
        title
        episodeID
      }
    }
  }
`)(function HomePageComponent(data) {
  const films = useMemo(
    () =>
      toSorted(data.allFilms?.films ?? [], (film1, film2) => {
        if (film1?.episodeID == null || film2?.episodeID == null) {
          throw new Error(
            'This API should not return null films or null episode IDs.',
          );
        }
        return film1.episodeID > film2.episodeID ? 1 : -1;
      }).filter(nonNullable),
    [data.allFilms?.films],
  );

  return (
    <>
      <h1>Star Wars Film Archive</h1>
      {films.map((film) => (
        <h2 key={film.id}>
          Episode {film.episodeID}: {film.title}
        </h2>
      ))}
    </>
  );
});
```

## Make a network request for the data that the `Root.HomePage` component needs

That Isograph component isn't doing much on its own. We need to fetch the server fields it requested.

In order to fetch the data, Isograph requires that you define an entrypoint. An entrypoint definition might look like ``iso(`entrypoint Root.HomePage`)``. When the Isograph compiler encounters an entrypoint definition, it generates a GraphQL query for all of the fields reachable from that field:

```graphql
query HomePage {
  allFilms {
    films {
      id
      episodeID
      title
    }
  }
}
```

So, create a file at `src/components/HomePageRoute.tsx`, and make its contents:

```tsx
import React from 'react';
import { useLazyReference } from '@isograph/react';
import { iso } from '@iso';

export default function HomePageRoute() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Root.HomePage`),
    {
      /* query variables */
    },
  );
  return null;
}
```

and change `src/pages/index.tsx` to be:

```tsx
import HomePageRoute from '@/components/HomePageRoute';

export default function Home() {
  return <HomePageRoute />;
}
```

The `useLazyReference` function will **make a network request** when it is first rendered.

So, whenever we render the `HomePageRoute` component, the Isograph runtime will make a network request for all the fields selected by the `Root.HomePage` component. Try it! If you navigate to `localhost:3000` and open the network tab, you'll see a network request that returns the fields we requested!

## Render the component

Now, we still need to render our `Query.HomePage` component. In order to do this, we call `useResult` to read the query reference. This gives us the value of that field (i.e. a component), which we can render.

```tsx
import React from 'react';
import { useLazyReference, useResult } from '@isograph/react';
import { iso } from '@iso';

export default function HomePageRoute() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Root.HomePage`),
    {
      /* query variables */
    },
  );
  const HomePage = useResult(fragmentReference);
  return <HomePage />;
}
```

Nice! Look at that beautiful list of Star Wars episodes!

## Add a subcomponent

A key principle of React is that you can divide your components into subcomponents. Let's do that! For example, create `src/components/EpisodeTitle.tsx` containing:

```tsx
import React from 'react';
import { iso } from '@iso';

export const EpisodeTitle = iso(`
  field Film.EpisodeTitle @component {
    title
    episodeID
  }
`)(function EpisodeTitleComponent(data) {
  return (
    <h2>
      Episode {data.episodeID}: {data.title}
    </h2>
  );
});
```

Let's use this component by modifying `HomePage.tsx` to be the following. Note the two new sections:

```tsx
import React, { useMemo } from 'react';
import { iso } from '@iso';

function nonNullable<T>(value: T): value is NonNullable<T> {
  return value != null;
}

function toSorted<T>(arr: T[], comparator: (a: T, b: T) => number): T[] {
  const sorted = [...arr];
  sorted.sort(comparator);
  return sorted;
}

export const HomePage = iso(`
  field Root.HomePage @component {
    allFilms {
      films {
        id
        episodeID
        EpisodeTitle
      }
    }
  }
`)(function HomePageComponent(data) {
  const films = useMemo(
    () =>
      toSorted(data.allFilms?.films ?? [], (film1, film2) => {
        if (film1?.episodeID == null || film2?.episodeID == null) {
          throw new Error(
            'This API should not return null films or null episode IDs.',
          );
        }
        return film1.episodeID > film2.episodeID ? 1 : -1;
      }).filter(nonNullable),
    [data.allFilms?.films],
  );

  return (
    <>
      <h1>Star Wars Film Archive</h1>
      {films.map((film) => (
        <film.EpisodeTitle key={film.id} />
      ))}
    </>
  );
});
```

Now, if you refresh, the UI will be divided into subcomponents and look exactly the same! Nice! 🎉

## Congratulations

Congratulations! You just built your first Isograph app.

Want more? Try extracting the sorted list of films into its own client field (no need to use `@component` for this one.) Use hooks in your components (they work!) Check out the [magic mutation fields](/docs/expose-field-directives/) documentation to learn about how Isograph lets you update your data.

Or, [join the Discord](https://discord.gg/rjDDwvZR)!
