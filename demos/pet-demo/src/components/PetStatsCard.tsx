import React from 'react';
import { iso } from '@iso';
import { Card, CardContent } from '@mui/material';

export const PetStatsCard = iso(`
  field Pet.PetStatsCard @component {
    id
    nickname
    __refetch
    age
    stats {
      weight
      intelligence
      cuteness
      hunger
      sociability
      energy
    }
  }
`)(function PetStatsCardComponent(data) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, cursor: 'pointer' }}
    >
      <CardContent>
        <h2>Stats</h2>
        <ul>
          <li>Weight: {data.stats.weight}</li>
          <li>Intelligence: {data.stats.intelligence}</li>
          <li>Cuteness: {data.stats.cuteness}</li>
          <li>Hunger: {data.stats.hunger}</li>
          <li>Sociability: {data.stats.sociability}</li>
          <li>Energy: {data.stats.energy}</li>
        </ul>
      </CardContent>
    </Card>
  );
});
