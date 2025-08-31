import { type Pet__Avatar__output_type } from '../../Pet/Avatar/output_type';
import { type Pet__FavoritePhraseLoader__output_type } from '../../Pet/FavoritePhraseLoader/output_type';

export type Pet__PetSummaryCard__param = {
  readonly data: {
    readonly id: string,
    readonly name: string,
    readonly Avatar: Pet__Avatar__output_type,
    readonly tagline: string,
    readonly FavoritePhraseLoader: Pet__FavoritePhraseLoader__output_type,
  },
  readonly parameters: Record<PropertyKey, never>,
};
