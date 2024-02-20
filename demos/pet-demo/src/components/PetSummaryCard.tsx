import React from 'react';
import { iso } from '@iso';
import { Avatar, Card, CardContent, Stack } from '@mui/material';

export const PetSummaryCard = iso(`
  field Pet.PetSummaryCard @component {
    id
    name
    picture
    tagline
    FavoritePhraseLoader
  }
`)(function PetSummaryCardComponent(props) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, cursor: 'pointer' }}
    >
      <CardContent>
        <Stack direction="row" spacing={4}>
          <Avatar
            sx={{ height: 100, width: 100 }}
            src={props.data.picture}
            onClick={() =>
              props.navigateTo({ kind: 'PetDetail', id: props.data.id })
            }
          />
          <div style={{ width: 300 }}>
            <h2>{props.data.name}</h2>
            <div>
              <i>{props.data.tagline}</i>
            </div>
            <props.data.FavoritePhraseLoader />
          </div>
        </Stack>
      </CardContent>
    </Card>
  );
});
