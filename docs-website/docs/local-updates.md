# Local updates

> This article is about making local updates. See also the [mutations](/docs/mutations) documentation for instructions on how to trigger network requests that update data on the backend.
>
> These APIs should be used in tandem. In the future, we will release an API that combines these.

## Big picture

### Local vs. remote updates

A traditional mutation works as follows:

- The app makes a network request, instructing the server to update some data.
- While this request is in flight, the app may show a loading indicator.
- That updated data is returned to the client, and written into the store. Components are re-rendered to reflect the updated data.

Now, if we refresh the page, we continue to see updated data.

A local update works as follows:

- The local Isograph store is updated with new data. Components are re-rendered to reflect the updated data.

Now, if we refresh the page, the changes are gone!

### When to use local updates

You should use local updates in several situations:

- When the data is indeed transitory, such as a `didSave` flag.
- In combination with mutations, i.e. to simulate optimistic updates.
- To keep some field up-to-date in response to server-sent events. (Isograph doesn't yet help you here; you should set those up manually on your own.)

## How

### `@updatable` and `startUpdate`

In selection sets, selections can be labeled `@updatable`.

:::warning
**Currently, we do not restrict `@updatable` to just valid selections**, but you should only annotate a field with `@updatable` if it is a "regular" data field, e.g. `name` or `best_friend`, and not if it is:

- `__typename`, `id` or `link`
- another resolver, e.g. `PetDetailCard`
- a refetch or `@exposeAs` field

Just use common sense!
:::

If at least one field is labeled `@updatable`, then the client field will receive a `startUpdate` function. You call `startUpdate` and pass another function, which receives an `updatableProxy`. You can mutate the `@updatable` fields in this proxy as you like:

```tsx
export const PetPhraseCard = iso(`
  field Pet.PetPhraseCard @component {
    name
    favorite_phrase @updatable
  }
`)(function PetPhraseCardComponent({ data, startUpdate }) {
  return (
    <>
      <p>
        {data.name}'s favorite phrase is:{' '}
        <b>&quot;{data.favorite_phrase}&quot;</b>
      </p>
      <Button
        onClick={() => {
          startUpdate(({ updatableData }) => {
            updatableData.favorite_phrase = 'GIVE ME KIBBLE';
            // The following line would give a TypeScript error, and not do anything at runtime:
            // updatableData.name = 'Oops';
          });
        }}
      >
        Update favorite phrase, locally
      </Button>
    </>
  );
});
```

### Updating object selections

The previous example showed how to update a scalar selection (`favorite_phrase`). To update an object, you must select `link` on the target object. (This is enforced via TypeScript).

```tsx
export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    p0: pet(id: 0) {
      stats @updatable {
        weight
      }
    }
    p1: pet(id: 1) {
      stats {
        weight
        link
      }
    }
  }
`)(function HomeRouteComponent({ data, startUpdate }) {
  return (
    <>
      p0: {data.p0?.stats?.weight}
      <br />
      p1: {data.p1?.stats?.weight}
      <Button
        onClick={() => {
          startUpdate(({ updatableData }) => {
            const p0 = updatableData.p0;
            const p1Stats = updatableData.p1?.stats;
            if (p0 == null || p1Stats == null) {return;}
            p0.stats = p1Stats;
          });
        }}
      >
        Set p0's stats to p1's stats. (What a weird example!)
      </Button>
    </>
  )
}
```

## In combination with mutations

Local updates can be used in combination with mutations to simulate optimistic updates. But make sure you roll them back in the mutation's `onError` callback.

Proper optimistic updates are in progress.

## FAQ

### Why `startUpdate`?

Why do all updates have to go through `startUpdate`? That's because, for performance, we need to know when all the updates have completed. This happens when the function passed to `startUpdate` has completed. At this point, we trigger re-renders in all affected components.

If we triggered a re-render on every change, instead of all at once, something like this would be tremendously inefficient:

```tsx
updatableData.firstName = 'Jeremy';
updatableData.lastName = 'Bentham';
// and so on for many other fields
```

### Async?

For the performance reasons listed above, we also cannot support async functions pass to `startUpdate`. But you can call `startUpdate` multiple times, if you like.
