import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { Route } from './router';
import { EntrypointReader, useClientSideDefer } from '@isograph/react';

export const PetDetailDeferredRoute = iso(`
  field Query.PetDetailDeferredRoute($id: ID!) @component {
    pet(id: $id) {
      name
      PetCheckinsCard @loadable
    }
  }
`)(function PetDetailRouteComponent(
  data,
  { navigateTo }: { navigateTo: (nextRoute: Route) => void },
) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  // eslint-disable-next-line react-hooks/rules-of-hooks
  const petCheckinsCard = useClientSideDefer(pet.PetCheckinsCard);

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
            <EntrypointReader queryReference={petCheckinsCard} />
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
});
