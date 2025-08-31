import { iso } from '@iso';
import { FragmentReader, useImperativeReference } from '@isograph/react';
import { Button, Card, CardContent } from '@mui/material';
import React, { Suspense } from 'react';

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
  const button = (
    <Button
      onClick={() => {
        loadMutation({
          input: {
            id: pet.id,
            tagline: getRandomTagline(),
          },
        });
      }}
      variant="contained"
    >
      Randomize tagline
    </Button>
  );

  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>
        <h2>Tagline</h2>
        <p>&quot;{pet.tagline}&quot;</p>
        {mutationRef === null ? (
          button
        ) : (
          <Suspense fallback="Updating tagline...">
            {button}
            <br />
            <FragmentReader fragmentReference={mutationRef} />
          </Suspense>
        )}
      </CardContent>
    </Card>
  );
});

export const setTagline = iso(`
  field Mutation.SetTagline($input: SetPetTaglineParams!) @component {
    set_pet_tagline(input: $input) {
      pet {
        tagline
      }
    }
  }
`)(({ data }) => {
  return (
    <>
      Nice! You updated the pet&apos;s tagline to{' '}
      {data.set_pet_tagline?.pet?.tagline}!
    </>
  );
});

function getRandomTagline(): string {
  const index = Math.floor(Math.random() * 4);

  return [
    'I AM HUNGRY',
    'LETS GO TO PARK',
    'Pet me now, human',
    'I am... SUPER DOG',
  ][index];
}
