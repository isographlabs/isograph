import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';
import type { Query__PetFavoritePhrase__parameters } from './parameters_type';

export type Query__PetFavoritePhrase__param = {
  readonly data: {
    /**
Fetch a pet by id
    */
    readonly pet: ({
      readonly fullName: Pet__fullName__output_type,
      /**
It's probably something from David Foster Wallace or Dostoevsky, keep up.
      */
      readonly favorite_phrase: (string | null),
    } | null),
  },
  readonly parameters: Query__PetFavoritePhrase__parameters,
};
