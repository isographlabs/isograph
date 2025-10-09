import { iso } from '@iso';
import { Button, Card, CardContent, Stack } from '@mui/material';
import React from 'react';
import { useNavigateTo } from '../routes';

export const PetSummaryCard = iso(`
  field Pet.PetSummaryCard @component {
    id
    fullName
    Avatar
    tagline
    FavoritePhraseLoader
  }
`)(function PetSummaryCardComponent({ data: pet }) {
  const navigateTo = useNavigateTo();
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <Stack direction="row" spacing={4}>
          <pet.Avatar
            onClick={() =>
              navigateTo({
                kind: 'PetDetail',
                id: pet.id,
              })
            }
          />
          <div style={{ width: 300 }}>
            <h2
              onClick={() =>
                navigateTo({
                  kind: 'PetDetailDeferred',
                  id: pet.id,
                })
              }
              style={{ cursor: 'pointer' }}
            >
              {pet.fullName}
            </h2>
            <div>
              <i>{pet.tagline}</i>
            </div>
            <pet.FavoritePhraseLoader />
            <div>
              <Button
                onClick={() =>
                  navigateTo({ kind: 'PetCheckinList', id: pet.id })
                }
                variant="text"
              >
                Checkins
              </Button>
            </div>
          </div>
        </Stack>
      </CardContent>
    </Card>
  );
});
