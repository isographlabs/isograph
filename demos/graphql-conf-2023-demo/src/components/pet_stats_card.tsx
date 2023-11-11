import React from "react";
import { iso } from "@isograph/react";
import { Card, CardContent } from "@mui/material";

import { ResolverParameterType as PetStatsCardParams } from "../__isograph/Pet/pet_stats_card/reader.isograph";

export const pet_stats_card = iso<
  PetStatsCardParams,
  ReturnType<typeof PetStatsCard>
>`
  Pet.pet_stats_card @component {
    id,
    nickname,
    __refetch,
    age,
    stats {
      weight,
      intelligence,
      cuteness,
      hunger,
      sociability,
      energy,
    },
  }
`(PetStatsCard);

function PetStatsCard(props: PetStatsCardParams) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, cursor: "pointer" }}
    >
      <CardContent>
        <h2>Stats</h2>
        <ul>
          <li>Weight: {props.data.stats.weight}</li>
          <li>Intelligence: {props.data.stats.intelligence}</li>
          <li>Cuteness: {props.data.stats.cuteness}</li>
          <li>Hunger: {props.data.stats.hunger}</li>
          <li>Sociability: {props.data.stats.sociability}</li>
          <li>Energy: {props.data.stats.energy}</li>
        </ul>
      </CardContent>
    </Card>
  );
}
