import React from 'react';
import { iso } from '@iso';
import { Container, Stack } from '@mui/material';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRouteComponent(props) {
  return (
    <Container maxWidth="md">
      <h1>Robert's Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {props.data.pets.map((pet) => (
          <pet.PetSummaryCard navigateTo={props.navigateTo} key={pet.id} />
        ))}
      </Stack>
    </Container>
  );
});
