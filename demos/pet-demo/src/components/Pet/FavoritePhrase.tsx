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

  return !!pet.favorite_phrase ? (
    <p>
      {pet.fullName} likes to say: &quot;{pet.favorite_phrase}&quot;
    </p>
  ) : (
    <p>{pet.fullName} has no favorite phrase!</p>
  );
});
