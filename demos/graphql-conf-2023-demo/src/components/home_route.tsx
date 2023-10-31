import React from "react";
import { iso, read, useLazyReference } from "@isograph/react";
import { Container, Stack } from "@mui/material";
import { type Route } from "./router";

import homeRouteQuery from "../__isograph/Query/home_route.isograph";

iso`
  Query.home_route @fetchable {
    pets {
      id,
      pet_summary_card,
    },
  }
`;

export function HomeRoute({ navigateTo }: { navigateTo: (path: Route) => void }) {
  const { queryReference } = useLazyReference(homeRouteQuery, {});
  const data = read(queryReference);

  return (
    <Container maxWidth="md">
      <h1>Robert's Pet List 3000</h1>
      <Stack direction="column" spacing={4}>
        {data.pets.map((pet) => (
          <React.Fragment key={pet.id}>
            {pet.pet_summary_card({
              navigateTo,
            })}
          </React.Fragment>
        ))}
      </Stack>
    </Container>
  );
}
