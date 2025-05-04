import { iso } from '@iso';
import { FragmentReader, useLazyReference } from '@isograph/react';
import { Container, Stack } from '@mui/material';
import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading } from './routes';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRouteComponent({ data }) {
  return (
    <Container maxWidth="md">
      <h1>Robert&apos;s Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {data.pets.map((pet) => (
          <pet.PetSummaryCard key={pet.id} />
        ))}
      </Stack>
    </Container>
  );
});

export function HomeRouteLoader() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.HomeRoute`),
    {},
    {
      shouldFetch: 'Yes',
    },
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
