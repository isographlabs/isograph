import { useImperativeReference } from '@isograph/react';
import { iso } from '../__isograph/iso';
import { Button } from '@mui/material';

export const setTagline = iso(`
  field Mutation.SetTaglineTest($input: SetPetTaglineParams!) {
    set_pet_tagline(input: $input) {
      pet {
        id
      }
    }

    TestLazyReference @loadable
  }
`)(() => {});

export const SomeThing = iso(`
    field Mutation.TestLazyReference {
      expose_field_on_mutation
    }`)(() => {
  return null;
});

export const SetTaglineTest = iso(`
    field Pet.PetTaglineTestCard @component {
        id
    }
    `)(({ data }) => {
  const {
    fragmentReference: mutationRef,
    loadFragmentReference: loadMutation,
  } = useImperativeReference(iso(`entrypoint Mutation.SetTaglineTest`));

  if (mutationRef === null) {
    return (
      <Button
        onClick={() => {
          loadMutation({
            input: {
              id: data.id,
              tagline: 'SUPER DOG',
            },
          });
        }}
        variant="contained"
      >
        Loadable Mutation Field
      </Button>
    );
  }
});
