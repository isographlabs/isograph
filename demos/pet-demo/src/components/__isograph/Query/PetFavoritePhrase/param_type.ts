
import { type Variables } from '@isograph/react';

export type Query__PetFavoritePhrase__param = {
  data: {
    pet: ({
      name: string,
      favorite_phrase: (string | null),
    } | null),
  },
  parameters: Variables,
};
