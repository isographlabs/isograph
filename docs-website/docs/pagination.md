# Pagination

Loadable fields can also be used to paginate.

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

You can also use `useConnectionSpecPagination` if your connection field conforms to the [Relay connection spec](https://facebook.github.io/relay/graphql/connections.htm).
