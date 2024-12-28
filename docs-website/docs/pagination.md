# Pagination

Loadable fields can also be used to paginate.

:::note
This documentation demonstrates the `useSkipLimitPagination` hook. You can also use `useConnectionSpecPagination` if your connection field conforms to the [Relay connection spec](https://facebook.github.io/relay/graphql/connections.htm).
:::

:::note
This API is likely to get simplified substantially, as we support `@loadable` on linked server fields.
:::

:::note
See [this file](https://github.com/isographlabs/isograph/blob/dc7beaeab163159a9b38dbe3cbd731f7a03b3e38/demos/pet-demo/src/components/Newsfeed/NewsfeedRoute.tsx) for a real-world example.
:::

## Walk through

First, define a client field (without `@component`) that returns an array of items and accepts `skip` and `limit` parameters. It can accept other params:

```tsx
import { iso } from '@iso';

export const PetCheckinsCardList = iso(`
  field Pet.PetCheckinsCardList($skip: Int, $limit: Int) {
    checkins(skip: $skip, limit: $limit) {
      CheckinDisplay
      id
    }
  }
`)(function PetCheckinsCardComponent({ data }) {
  return data.checkins;
});
```

Next, select that field loadably.

```tsx
import { iso } from '@iso';

export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetCheckinListRoute($id: ID!) @component {
    pet(id: $id) {
      PetCheckinsCardList @loadable(lazyLoadArtifact: true)
    }
  }
`)(function PetDetailRouteComponent({ data }) {});
```

Next, pass that loadable field to `useSkipLimitPagination`:

```tsx
import { iso } from '@iso';

export const PetDetailDeferredRouteComponent = iso(`
  field Pet.PetCheckins @component {
    PetCheckinsCardList @loadable(lazyLoadArtifact: true)
  }
`)(function PetDetailRouteComponent(pet) {
  const skipLimitPaginationState = useSkipLimitPagination(
    pet.PetCheckinsCardList,
  );

  return (
    <>
      <Button
        onClick={() =>
          skipLimitPaginationState.kind === 'Complete'
            ? skipLimitPaginationState.fetchMore(undefined, count)
            : null
        }
        disabled={skipLimitPaginationState.kind === 'Pending'}
        variant="contained"
      >
        Load more
      </Button>

      {skipLimitPaginationState.results.map((item) => (
        <div key={item.id}>
          <item.CheckinDisplay />
        </div>
      ))}
      {skipLimitPaginationState.kind === 'Pending' && <div>Loading...</div>}
    </>
  );
});
```

## Prefetching initial pages

Each pagination hook accepts a second `initialState` parameter. You can fetch data as part of the parent query (or anywhere, in fact!) and pass the appropriate value to that `initialState`. Consider:

```jsx
export const Newsfeed = iso(`
  field Query.Newsfeed @component {
    viewer {
      newsfeed(skip: 0, limit: 6) {
        NewsfeedAdOrBlog
      }
      NewsfeedPaginationComponent @loadable
    }
  }
`)(function PetDetailRouteComponent({ data }) {
  const viewer = data.viewer;

  const paginationState = useSkipLimitPagination(
    viewer.NewsfeedPaginationComponent,
    { skip: viewer.newsfeed.length },
  );

  const newsfeedItems = viewer.newsfeed.concat(paginationState.results);

  // ...
});
```

## Data-driven dependencies

Check out the [data driven dependencies](/docs/data-driven-dependencies/) documentation to see how to combine [`@loadable` fields](/docs/loadable-fields/), pagination and [`asConcreteType` fields](/docs/abstract-types/) to fetch the minimal amount of data and JavaScript needed!
