import React from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';

export const PetStatsCard = iso(`
  field Pet.PetStatsCard @component {
    id
    nickname
    age
    stats {
      weight
      intelligence
      cuteness
      hunger
      sociability
      energy
      refetch_pet_stats(id: $id)
    }
  }
`)(function PetStatsCardComponent({ data: pet }) {
  return (
    <Card
      variant="outlined"
      sx={{
        width: 450,
        boxShadow: 3,
        cursor: 'pointer',
        backgroundColor: '#BBB',
      }}
    >
      <CardContent>
        <h2>Stats</h2>
        {pet.stats ? (
          <ul>
            <li>Weight: {pet.stats.weight}</li>
            <li>Intelligence: {pet.stats.intelligence}</li>
            <li>Cuteness: {pet.stats.cuteness}</li>
            <li>Hunger: {pet.stats.hunger}</li>
            <li>Sociability: {pet.stats.sociability}</li>
            <li>Energy: {pet.stats.energy}</li>
          </ul>
        ) : null}
        <Button
          variant="contained"
          onClick={() => pet.stats?.refetch_pet_stats({ id: pet.id })[1]()}
        >
          Refetch pet
        </Button>
      </CardContent>
    </Card>
  );
});
