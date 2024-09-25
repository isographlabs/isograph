import React from 'react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';
import { useImperativeReference } from '@isograph/react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';

export const PetTaglineCard = iso(`
field Pet.PetTaglineCard @component {
  id
  tagline
}
`)(function PetTaglineCardComponent({ data: pet }) {
  const {
    fragmentReference: mutationRef,
    loadFragmentReference: loadMutation,
  } = useImperativeReference(iso(`entrypoint Mutation.SetTagline`));

  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <h2>Tagline</h2>
        <p>&quot;{pet.tagline}&quot;</p>
        {mutationRef == UNASSIGNED_STATE ? (
          <Button
            onClick={() => {
              loadMutation({
                input: {
                  id: pet.id,
                  tagline: 'SUPER DOG',
                },
              });
            }}
            variant="contained"
          >
            Set tagline to SUPER DOG
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
});

export const setTagline = iso(`
  field Mutation.SetTagline($input: SetPetTaglineParams!) {
    set_pet_tagline(input: $input) {
      pet {
        tagline
      }
    }
  }
`)(() => {});
