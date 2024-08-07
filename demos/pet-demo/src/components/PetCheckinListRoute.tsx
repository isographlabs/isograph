import React, { useState } from 'react';
import { iso } from '@iso';
import { Container, Stack, Input, Button } from '@mui/material';
import {
  FragmentReader,
  useLazyReference,
  useSuspensefulSkipLimitPagination,
} from '@isograph/react';
import { FullPageLoading, PetCheckinListRoute, useNavigateTo } from './routes';
import { ErrorBoundary } from './ErrorBoundary';

export const PetDetailDeferredRouteComponent = iso(`
  field Query.PetCheckinListRoute($id: ID!) @component {
    pet(id: $id) {
      name
      PetCheckinsCardList @loadable
    }
  }
`)(function PetDetailRouteComponent(data) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  const navigateTo = useNavigateTo();

  const { fetchMore, results } = useSuspensefulSkipLimitPagination(
    pet.PetCheckinsCardList,
  );
  const [count, setCount] = useState(2);

  return (
    <Container maxWidth="md">
      <Stack direction="column" spacing={4}>
        <h1>{pet.name} Checkins List</h1>
        <h3
          onClick={() => navigateTo({ kind: 'Home' })}
          style={{ cursor: 'pointer' }}
        >
          ← Home
        </h3>
        <div>
          <Button
            onClick={() => fetchMore(undefined, count)}
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

        {results.map((item) => (
          <div>
            <item.CheckinDisplay />
          </div>
        ))}
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
