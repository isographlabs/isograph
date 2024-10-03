import { type Pet__FavoritePhraseLoader__output_type } from '../../Pet/FavoritePhraseLoader/output_type';

import { type Variables } from '@isograph/react';

export type Pet__PetSummaryCard__param = {
  readonly data: {
    readonly id: string,
    readonly name: string,
    readonly picture: string,
    readonly tagline: string,
    readonly FavoritePhraseLoader: Pet__FavoritePhraseLoader__output_type,
  },
  readonly parameters: Variables,
};
