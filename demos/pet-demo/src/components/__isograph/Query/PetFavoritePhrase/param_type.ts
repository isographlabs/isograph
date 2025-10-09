import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';
import type { Query__PetFavoritePhrase__parameters } from './parameters_type';

export type Query__PetFavoritePhrase__param = {
  readonly data: {
    readonly pet: ({
      readonly fullName: Pet__fullName__output_type,
      readonly favorite_phrase: (string | null),
    } | null),
  },
  readonly parameters: Query__PetFavoritePhrase__parameters,
};
