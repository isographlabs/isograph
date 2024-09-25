import React from 'react';
import { iso } from '@iso';
import { Card, CardContent } from '@mui/material';

export const PetPhraseCard = iso(`
field Pet.PetPhraseCard @component {
  id
  favorite_phrase
}
`)(function PetPhraseCardComponent({ data }) {
  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <h2>Likes to say</h2>
        <p>&quot;{data.favorite_phrase}&quot;</p>
      </CardContent>
    </Card>
  );
});
