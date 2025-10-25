import { iso } from '@iso';
import { FragmentRenderer, useImperativeReference } from '@isograph/react';
import { Button, Card, CardContent } from '@mui/material';
import React, { Suspense } from 'react';

export const PetTaglineCard = iso(`
  field Pet.PetTaglineCard @component {
    id
    tagline @updatable
  }
`)(function PetTaglineCardComponent({ data: pet, startUpdate }) {
  const {
    fragmentReference: mutationRef,
    loadFragmentReference: loadMutation,
  } = useImperativeReference(iso(`entrypoint Mutation.SetTagline`));
  const button = (
    <Button
      onClick={() => {
        const oldTagline = pet.tagline;
        const newTagline = getRandomTagline();

        startUpdate(({ updatableData }) => {
          updatableData.tagline = newTagline;
        });
        loadMutation(
          {
            input: {
              id: pet.id,
              tagline: newTagline,
            },
          },
          {
            onError: () => {
              console.log('Reverting');
              startUpdate(({ updatableData }) => {
                updatableData.tagline = oldTagline;
              });
            },
            shouldFetch: 'Yes',
          },
        );
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
            <FragmentRenderer fragmentReference={mutationRef} />
          </Suspense>
        )}
      </CardContent>
    </Card>
  );
});

export const setTagline = iso(`
  field Mutation.SetTagline(
    $input: SetPetTaglineParams !
  ) @component {
    set_pet_tagline(
      input: $input
    ) {
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
  const index = Math.floor(Math.random() * 8);

  return [
    'I AM HUNGRY',
    'LETS GO TO PARK',
    'Pet me now, human',
    'I am... SUPER DOG',
    'Woof',
    'Ruff',
    'Rub my belly',
    "I'm a good dog, aren't I?",
  ][index];
}
