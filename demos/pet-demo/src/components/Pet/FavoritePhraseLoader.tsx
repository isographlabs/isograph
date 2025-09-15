import { iso } from '@iso';
import { FragmentRenderer, useImperativeReference } from '@isograph/react';
import React from 'react';
import { ErrorBoundary } from '../ErrorBoundary';

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
            <FragmentRenderer fragmentReference={fragmentReference} />
          </React.Suspense>
        </ErrorBoundary>
      )}
    </>
  );
});
