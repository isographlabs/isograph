import React from 'react';
import { iso } from '@iso';
import { Avatar, Card, CardContent, Stack } from '@mui/material';
import { Route } from './router';

export const PetSummaryCard = iso(`
  field Pet.PetSummaryCard @component {
    id
    name
    picture
    tagline
    FavoritePhraseLoader
  }
`)(function PetSummaryCardComponent(
  data,
  runtimeProps: { navigateTo: (newRoute: Route) => void },
) {
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
              runtimeProps.navigateTo({
                kind: 'PetDetail',
                id: data.id,
              })
            }
          />
          <div style={{ width: 300 }}>
            <h2
              onClick={() =>
                runtimeProps.navigateTo({
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
          </div>
        </Stack>
      </CardContent>
    </Card>
  );
});
