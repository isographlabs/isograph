import React from "react";
import { iso } from "@isograph/react";
import { Container, Stack } from "@mui/material";
import { ResolverParameterType as PetDetailRouteParams } from "@iso/Query/pet_detail_route/reader.isograph";

export const pet_detail_route = iso<
  PetDetailRouteParams,
  ReturnType<typeof PetDetailRouteComponent>
>`
  Query.pet_detail_route($id: ID!) @component {
    pet(id: $id) {
      name,
      pet_checkins_card,
      pet_best_friend_card,
      pet_phrase_card,
    },
  }
`(PetDetailRouteComponent);

export function PetDetailRouteComponent({
  data,
  navigateTo,
}: PetDetailRouteParams) {
  return (
    <Container maxWidth="md">
      <h1>Pet Detail for {data.pet?.name}</h1>
      <h3
        onClick={() => navigateTo({ kind: "Home" })}
        style={{ cursor: "pointer" }}
      >
        ← Home
      </h3>
      <React.Suspense fallback={<h2>Loading pet details...</h2>}>
        <Stack direction="row" spacing={4}>
          {data.pet?.pet_checkins_card({ navigateTo })}
          <Stack direction="column" spacing={4}>
            {data.pet?.pet_best_friend_card({})}
            {data.pet?.pet_phrase_card({})}
          </Stack>
        </Stack>
      </React.Suspense>
    </Container>
  );
}
