import { iso } from '@iso';
import React from 'react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';
import { EntrypointReader, useImperativeReference } from '@isograph/react';

export const FavoritePhraseLoader = iso(`
  field Pet.FavoritePhraseLoader @component {
    id
  }
`)((pet) => {
  const { queryReference, loadQueryReference } = useImperativeReference(
    iso(`entrypoint Query.PetFavoritePhrase`),
  );

  return (
    <>
      {queryReference == UNASSIGNED_STATE ? (
        <button onClick={() => loadQueryReference({ id: pet.id })}>
          Reveal favorite phrase
        </button>
      ) : (
        <React.Suspense fallback="Loading favorite phrase...">
          <EntrypointReader queryReference={queryReference} />
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
