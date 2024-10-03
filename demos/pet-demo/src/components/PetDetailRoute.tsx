import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { FragmentReader, useLazyReference } from '@isograph/react';
import { FullPageLoading, PetDetailRoute, useNavigateTo } from './routes';
import { ErrorBoundary } from './ErrorBoundary';

export const PetDetailRouteComponent = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    PetDetailRouteInner(actualId: $id) 
  }
`)(function PetDetailRouteComponent({ data }) {
  return <data.PetDetailRouteInner />;
});

export const PetDetailRouteInner = iso(`
  field Query.PetDetailRouteInner($actualId: ID!) @component {
    pet(id: $actualId) {
      name
      PetCheckinsCard
      PetBestFriendCard
      PetPhraseCard
      PetTaglineCard
      PetStatsCard
    }
  }
`)(function PetDetailRouteComponentInner({ data }) {
  const navigateTo = useNavigateTo();
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }
  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {pet.name}</h1>
      <h3
        onClick={() => navigateTo({ kind: 'Home' })}
        style={{ cursor: 'pointer' }}
      >
        ← Home
      </h3>
      <React.Suspense fallback={<h2>Loading pet details...</h2>}>
        <Stack direction="row" spacing={4}>
          <Stack direction="column" spacing={4}>
            <pet.PetCheckinsCard />
            <pet.PetStatsCard />
          </Stack>
          <Stack direction="column" spacing={4}>
            <pet.PetBestFriendCard />

            <pet.PetPhraseCard />
            <pet.PetTaglineCard />
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
});

export function PetDetailRouteLoader({ route }: { route: PetDetailRoute }) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PetDetailRoute`),
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
