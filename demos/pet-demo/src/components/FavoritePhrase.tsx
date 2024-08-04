import { iso } from '@iso';
import React from 'react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';
import { FragmentReader, useImperativeReference } from '@isograph/react';

export const FavoritePhraseLoader = iso(`
  field Pet.FavoritePhraseLoader @component {
    id
  }
`)((pet) => {
  const { fragmentReference, loadFragmentReference } = useImperativeReference(
    iso(`entrypoint Query.PetFavoritePhrase`),
  );

  return (
    <>
      {fragmentReference == UNASSIGNED_STATE ? (
        <button onClick={() => loadFragmentReference({ id: pet.id })}>
          Reveal favorite phrase
        </button>
      ) : (
        <React.Suspense fallback="Loading favorite phrase...">
          <FragmentReader fragmentReference={fragmentReference} />
        </React.Suspense>
      )}
    </>
  );
});

export const PetFavoritePhrase = iso(`
  field Query.PetFavoritePhrase($id: ID!) @component {
    pet(id: $id) {
      name
      favorite_phrase
    }
  }
`)((data) => {
  const pet = data.pet;
  if (pet == null) return;
  return (
    <p>
      {pet.name} likes to say: &quot;{pet.favorite_phrase}&quot;
    </p>
  );
});
