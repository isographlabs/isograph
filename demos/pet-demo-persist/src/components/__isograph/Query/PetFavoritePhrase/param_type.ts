import type { Query__PetFavoritePhrase__parameters } from './parameters_type';

export type Query__PetFavoritePhrase__param = {
  readonly data: {
    readonly pet: ({
      readonly name: string,
      readonly favorite_phrase: (string | null),
    } | null),
  },
  readonly parameters: Query__PetFavoritePhrase__parameters,
};
