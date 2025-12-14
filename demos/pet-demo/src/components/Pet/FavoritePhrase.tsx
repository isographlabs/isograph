import { iso } from '@iso';
import React from 'react';

console.log('Query.PetFavoritePhrase bundle being loaded!');

export const PetFavoritePhrase = iso(`
  field Query.PetFavoritePhrase(
    $id: ID !
  ) @component {
    pet(
      id: $id
    ) {
      fullName
      favorite_phrase
    }
  }
`)(({ data }) => {
  const pet = data.pet;
  if (pet == null) return;

  return pet.favorite_phrase != null ? (
    <p>
      {pet.fullName} likes to say: &quot;{pet.favorite_phrase}&quot;
    </p>
  ) : (
    <p>{pet.fullName} has no favorite phrase!</p>
  );
});

export const PetFavoritePhrase2 = iso(`
  field Query.PetFavoritePhrase2(
    $id: ID !
  ) @component
  """
   PetFavoritePhrase2 because we currently have a bug where we don't catch the fact
   that an entrypoint generated via @loadable and a regular entrypoint need to have
   identical @lazyLoad settings. Oops!
   """
  {
    pet(
      id: $id
    ) {
      fullName
      favorite_phrase
    }
  }
`)(({ data }) => {
  const pet = data.pet;
  if (pet == null) return;

  return pet.favorite_phrase != null ? (
    <p>
      {pet.fullName} likes to say: &quot;{pet.favorite_phrase}&quot;
    </p>
  ) : (
    <p>{pet.fullName} has no favorite phrase!</p>
  );
});
