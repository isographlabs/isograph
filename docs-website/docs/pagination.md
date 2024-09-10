# Pagination

Loadable fields can also be used to paginate.

:::note
This API is likely to get simplified substantially, as we support `@loadable` on linked server fields. We will also add support for Relay-style pagination.
:::

## Walk through

Define a client field (without `@component`) that returns an array of items and accepts `skip` and `limit` parameters. It can accept other params:

```tsx
import { iso } from '@iso';

export const PetCheckinsCardList = iso(`
  field Pet.PetCheckinsCardList($skip: Int, $limit: Int) {
    checkins(skip: $skip, limit: $limit) {
      CheckinDisplay
      id
    }
  }
`)(function PetCheckinsCardComponent(data) {
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
`)(function PetDetailRouteComponent(data) {});
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
