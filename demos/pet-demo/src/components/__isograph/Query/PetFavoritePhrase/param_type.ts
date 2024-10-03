
import { type Variables } from '@isograph/react';

export type Query__PetFavoritePhrase__param = {
  readonly data: {
    readonly pet: ({
      readonly name: string,
      readonly favorite_phrase: (string | null),
    } | null),
  },
  readonly parameters: Variables,
};
