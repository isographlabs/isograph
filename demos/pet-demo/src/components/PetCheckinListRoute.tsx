import React, { useState } from 'react';
import { iso } from '@iso';
import { Container, Stack, Input, Button } from '@mui/material';
import {
  FragmentReader,
  useLazyReference,
  useSkipLimitPagination,
} from '@isograph/react';
import { FullPageLoading, PetCheckinListRoute, useNavigateTo } from './routes';
import { ErrorBoundary } from './ErrorBoundary';

export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetCheckinListRoute($id: ID!) @component {
    pet(id: $id) {
      FirstCheckinMakeSuperButton
      name
      PetCheckinsCardList @loadable(lazyLoadArtifact: true)
    }
  }
`)(function PetDetailRouteComponent({ data }) {
  const { pet } = data;
  const navigateTo = useNavigateTo();

  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  // eslint-disable-next-line react-hooks/rules-of-hooks
  const skipLimitPaginationState = useSkipLimitPagination(
    pet.PetCheckinsCardList,
  );
  // eslint-disable-next-line react-hooks/rules-of-hooks
  const [count, setCount] = useState(2);

  return (
    <Container maxWidth="md">
      <Stack direction="column" spacing={4}>
        <h1>{pet.name} Checkins List</h1>
        <pet.FirstCheckinMakeSuperButton />
        <h3
          onClick={() => navigateTo({ kind: 'Home' })}
          style={{ cursor: 'pointer' }}
        >
          ← Home
        </h3>
        <div>
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
          <span style={{ width: 20, display: 'inline-block' }} />
          How many? <span style={{ width: 20, display: 'inline-block' }} />
          <Input
            value={count}
            onChange={(e) => setCount(Number(e.target.value))}
            type="number"
          />
        </div>

        {skipLimitPaginationState.results.map((item) => (
          <div key={item.id}>
            <item.CheckinDisplay />
          </div>
        ))}
        {skipLimitPaginationState.kind === 'Pending' && <div>Loading...</div>}
      </Stack>
    </Container>
  );
});

export function PetCheckinListLoader({
  route,
}: {
  route: PetCheckinListRoute;
}) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PetCheckinListRoute`),
    { id: route.id },
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentReader
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}
