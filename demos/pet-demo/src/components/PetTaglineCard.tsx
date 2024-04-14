import React from 'react';
import { iso } from '@iso';
import { Card, CardContent } from '@mui/material';

export const PetTaglineCard = iso(`
field Pet.PetTaglineCard @component {
  id
  tagline
}
`)(function PetTaglineCardComponent(data) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Tagline</h2>
        <p>&quot;{data.tagline}&quot;</p>
      </CardContent>
    </Card>
  );
});
