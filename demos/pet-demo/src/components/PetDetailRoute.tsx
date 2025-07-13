import { iso } from '@iso';
import {
  FragmentReader,
  normalizeData,
  ROOT_ID,
  useIsographEnvironment,
  useLazyReference,
} from '@isograph/react';
import { Button, Card, CardContent, Container, Stack } from '@mui/material';
import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading, PetDetailRoute, useNavigateTo } from './routes';

export const PetDetailRouteComponent = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    pet(id: $id) {
      custom_pet_refetch
      name
      PetCheckinsCard
      PetBestFriendCard
      PetPhraseCard
      PetTaglineCard
      PetStatsCard(id: $id)
    }
  }
`)(function PetDetailRouteComponent({ data, parameters }) {
  const env = useIsographEnvironment();
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
      <h3
        onClick={() => {
          normalizeData(
            env,
            iso(` entrypoint Query.Foo `).networkRequestInfo.normalizationAst
              .selections,
            {
              pet____id___v_id: {
                id: parameters.id,
                __typename: 'Pet',
                name: 'POOPFACE',
              },
            },
            {
              id: parameters.id,
            },
            [],
            {
              __link: ROOT_ID,
              __typename: 'Query',
            },
          );
        }}
        style={{ cursor: 'pointer' }}
      >
        ← Set name to POOPFACE
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

            <Card
              variant="outlined"
              sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
            >
              <CardContent>
                <Button
                  onClick={() =>
                    pet.custom_pet_refetch({ id: parameters.id })[1]()
                  }
                  variant="contained"
                >
                  Refetch pet
                </Button>
              </CardContent>
            </Card>
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
    {
      onComplete: (data) => {
        console.log(
          'The Query.PetDetailRoute network request has completed.',
          data,
        );
      },
      onError: () => {
        console.log('The Query.PetDetailRoute network request errored out.');
      },
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
