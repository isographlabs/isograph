import React from "react";
import { iso, read, useLazyReference } from "@isograph/react";
import { Container, Stack } from "@mui/material";

import petDetailRouteQuery from "./__isograph/Query/pet_detail_route.isograph";
import { PetDetailRoute, Route } from "./router";

iso`
  Query.pet_detail_route($id: ID!) @fetchable {
    pet(id: $id) {
      name,
      pet_checkins_card,
      pet_best_friend_card,
      pet_phrase_card,
    },
  }
`;

export function PetDetailRoute({
  route,
  navigateTo,
}: {
  route: PetDetailRoute;
  navigateTo: (route: Route) => void;
}) {
  const { queryReference } = useLazyReference(petDetailRouteQuery, {
    id: route.id,
  });
  const data = read(queryReference);

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
