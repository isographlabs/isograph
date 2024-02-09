import React from 'react';
import { iso } from '@iso';
import { Card, CardContent } from '@mui/material';

export const PetPhraseCard = iso(`
field Pet.PetPhraseCard @component {
  id
  favorite_phrase
}
`)(function PetPhraseCardComponent(props) {
  return (
    <Card variant="outlined" sx={{ width: 450, boxShadow: 3 }}>
      <CardContent>
        <h2>Likes to say</h2>
        <p>"{props.data.favorite_phrase}"</p>
      </CardContent>
    </Card>
  );
});
