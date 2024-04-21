import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';
import { Route } from './router';

export const petSuperName = iso(`
  field Pet.petSuperName {
    name
  }
`)((pet) => `super ${pet.name}`);

export const HomeRoute = iso(`
  field Query.HomeRoute($id: ID!) @component {
    pets {
      id
      PetSummaryCard
    }
    pet(id: $id) {
      petSuperName
    }
  }
`)(function HomeRouteComponent(
  data,
  secondParam: { navigateTo: (newRoute: Route) => void },
) {
  console.log('home route data', { data });
  return (
    <Container maxWidth="md">
      <h1>Robert&apos;s Pet List 3000</h1> <span>{data.pet?.petSuperName}</span>
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
