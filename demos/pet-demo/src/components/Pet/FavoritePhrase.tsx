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
