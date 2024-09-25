import React from 'react';
import { iso } from '@iso';
import { Avatar, Button, Card, CardContent, Stack } from '@mui/material';
import { useNavigateTo } from './routes';

export const PetSummaryCard = iso(`
  field Pet.PetSummaryCard @component {
    id
    name
    picture
    tagline
    FavoritePhraseLoader
  }
`)(function PetSummaryCardComponent({ data }) {
  const navigateTo = useNavigateTo();
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <Stack direction="row" spacing={4}>
          <Avatar
            sx={{ height: 100, width: 100, cursor: 'pointer' }}
            src={data.picture}
            onClick={() =>
              navigateTo({
                kind: 'PetDetail',
                id: data.id,
              })
            }
          />
          <div style={{ width: 300 }}>
            <h2
              onClick={() =>
                navigateTo({
                  kind: 'PetDetailDeferred',
                  id: data.id,
                })
              }
              style={{ cursor: 'pointer' }}
            >
              {data.name}
            </h2>
            <div>
              <i>{data.tagline}</i>
            </div>
            <data.FavoritePhraseLoader />
            <div>
              <Button
                onClick={() =>
                  navigateTo({ kind: 'PetCheckinList', id: data.id })
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
