import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { Route } from './router';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRouteComponent(
  data,
  secondParam: { navigateTo: (newRoute: Route) => void },
) {
  return (
    <Container maxWidth="md">
      <h1>Robert&apos;s Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {data.pets.map((pet) => (
          <pet.PetSummaryCard
            navigateTo={secondParam.navigateTo}
            key={pet.id}
          />
        ))}
      </Stack>
    </Container>
  );
});
