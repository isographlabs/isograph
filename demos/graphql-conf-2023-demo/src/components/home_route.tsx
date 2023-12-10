import React from "react";
import { iso } from "@isograph/react";
import { Container, Stack } from "@mui/material";
import { ResolverParameterType as HomeRouteParams } from "@iso/Query/home_route/reader.isograph";

export const home_route = iso<HomeRouteParams, ReturnType<typeof HomeRoute>>`
  Query.home_route @component {
    pets {
      id,
      pet_summary_card,
    },
  }
`(HomeRoute);

function HomeRoute(props: HomeRouteParams) {
  return (
    <Container maxWidth="md">
      <h1>Robert's Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {props.data.pets.map((pet) => (
          <React.Fragment key={pet.id}>
            {pet.pet_summary_card({
              navigateTo: props.navigateTo,
            })}
          </React.Fragment>
        ))}
      </Stack>
    </Container>
  );
}
