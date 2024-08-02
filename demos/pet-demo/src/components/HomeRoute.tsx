import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { useLazyReference, useResult } from '@isograph/react';

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
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.HomeRoute`),
    {},
  );

  const Component = useResult(fragmentReference);
  return <Component />;
}
