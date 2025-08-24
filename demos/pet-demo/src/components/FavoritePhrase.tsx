import { iso } from '@iso';
import { FragmentReader, useImperativeReference } from '@isograph/react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';
import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';

export const FavoritePhraseLoader = iso(`
  field Pet.FavoritePhraseLoader @component {
    id
  }
`)(({ data: pet }) => {
  const { fragmentReference, loadFragmentReference } = useImperativeReference(
    iso(`entrypoint Query.PetFavoritePhrase @lazyLoad`),
  );

  return (
    <>
      {fragmentReference == null ? (
        <button
          onClick={() =>
            loadFragmentReference(
              { id: pet.id },
              {
                shouldFetch: 'Yes',
                onComplete: (data) => {
                  console.log(
                    'Successfully loaded favorite phrase and received this component',
                    data,
                  );
                },
                onError: () => {
                  console.log('Error when loading favorite phrase');
                },
              },
            )
          }
        >
          Reveal favorite phrase
        </button>
      ) : (
        <ErrorBoundary>
          <React.Suspense fallback="Loading favorite phrase...">
            <FragmentReader fragmentReference={fragmentReference} />
          </React.Suspense>
        </ErrorBoundary>
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
`)(({ data }) => {
  const pet = data.pet;
  if (pet == null) return;

  return !!pet.favorite_phrase ? (
    <p>
      {pet.name} likes to say: &quot;{pet.favorite_phrase}&quot;
    </p>
  ) : (
    <p>{pet.name} has no favorite phrase!</p>
  );
});
