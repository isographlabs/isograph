import React from 'react';
import { iso } from '@isograph/react';
import { Container, Stack } from '@mui/material';
import { ResolverParameterType as HomeRouteParams } from '@iso/Query/HomeRoute/reader';

export const HomeRoute = iso<HomeRouteParams>`
  field Query.HomeRoute @component {
    pets {
      id,
      PetSummaryCard,
    },
  }
`(HomeRouteComponent);

function HomeRouteComponent(props: HomeRouteParams) {
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
}
