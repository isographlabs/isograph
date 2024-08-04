import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import {
  FragmentReader,
  NetworkErrorReader,
  useLazyReference,
} from '@isograph/react';
import { FullPageLoading } from './routes';
import { ErrorBoundary } from './ErrorBoundary';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRouteComponent(data) {
  return (
    <Container maxWidth="md">
      <h1>Robert&apos;s Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {data.pets.map((pet) => (
          // the key is not needed here!
          // eslint-disable-next-line react/jsx-key
          <pet.PetSummaryCard />
        ))}
      </Stack>
    </Container>
  );
});

export function HomeRouteLoader() {
  const { fragmentReference, networkRequestReference } = useLazyReference(
    iso(`entrypoint Query.HomeRoute`),
    {},
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <NetworkErrorReader networkRequestReference={networkRequestReference}>
          <FragmentReader fragmentReference={fragmentReference} />
        </NetworkErrorReader>
      </React.Suspense>
    </ErrorBoundary>
  );
}
