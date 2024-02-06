import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { ResolverParameterType as PetDetailRouteParams } from '@iso/Query/PetDetailRoute/reader';

export const PetDetailRoute = iso(`
  field Query.PetDetailRoute($id: ID!) @component {
    pet(id: $id) {
      name,
      PetCheckinsCard,
      PetBestFriendCard,
      PetPhraseCard,
      PetTaglineCard,
    },
  }
`)(PetDetailRouteComponent);

export function PetDetailRouteComponent({
  data,
  navigateTo,
}: PetDetailRouteParams) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }
  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {data.pet?.name}</h1>
      <h3
        onClick={() => navigateTo({ kind: 'Home' })}
        style={{ cursor: 'pointer' }}
      >
        ← Home
      </h3>
      <React.Suspense fallback={<h2>Loading pet details...</h2>}>
        <Stack direction="row" spacing={4}>
          <pet.PetCheckinsCard navigateTo={navigateTo} />
          <Stack direction="column" spacing={4}>
            <pet.PetBestFriendCard />

            <pet.PetPhraseCard />
            <pet.PetTaglineCard />
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
}
