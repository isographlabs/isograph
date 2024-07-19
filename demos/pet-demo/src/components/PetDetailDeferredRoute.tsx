import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { Route } from './router';

export const PetDetailDeferredRoute = iso(`
  field Query.PetDetailDeferredRoute($id: ID!) @component {
    pet(id: $id) {
      name
      PetCheckinsCard
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
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
});
