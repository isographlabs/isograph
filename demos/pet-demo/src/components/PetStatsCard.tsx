import React from 'react';
import { iso } from '@isograph/react';
import { Card, CardContent } from '@mui/material';

import { ResolverParameterType as PetStatsCardParams } from '@iso/Pet/PetStatsCard/reader';

export const PetStatsCard = iso<PetStatsCardParams>`
  field Pet.PetStatsCard @component {
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
`(PetStatsCardComponent);

function PetStatsCardComponent(props: PetStatsCardParams) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, cursor: 'pointer' }}
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
