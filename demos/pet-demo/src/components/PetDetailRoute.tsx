import { iso } from '@iso';
import { FragmentRenderer, useLazyReference } from '@isograph/react';
import { Button, Card, CardContent, Container, Stack } from '@mui/material';
import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import type { PetDetailRoute } from './routes';
import { FullPageLoading, useNavigateTo } from './routes';

export const PetDetailRouteComponent = iso(`
  Query .PetDetailRoute($
    id:ID ! )
  @ component{ pet
    (id
      :$ id)
    { custom_pet_refetch
      fullName
      PetCheckinsCard
      PetBestFriendCard
      PetPhraseCard
      PetTaglineCard
      MutualBestFriendSetter
      PetStatsCard
      (id
        :$ id)
      }
    }
  
`)(function PetDetailRouteComponent({ data, parameters }) {
  const navigateTo = useNavigateTo();
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }
  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {pet.fullName}</h1>
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
            <pet.MutualBestFriendSetter />

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
    iso(`Query .PetDetailRoute`),
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
        <FragmentRenderer
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}
