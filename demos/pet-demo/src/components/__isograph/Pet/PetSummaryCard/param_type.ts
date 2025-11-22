import { type Pet__Avatar__output_type } from '../../Pet/Avatar/output_type';
import { type Pet__FavoritePhraseLoader__output_type } from '../../Pet/FavoritePhraseLoader/output_type';
import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';

export type Pet__PetSummaryCard__param = {
  readonly data: {
    /**
The pet's ID
    */
    readonly id: string,
    readonly fullName: Pet__fullName__output_type,
    /**
A picture of a pet, framed.
    */
    readonly Avatar: Pet__Avatar__output_type,
    /**
If your pet was a superhero.
    */
    readonly tagline: string,
    readonly FavoritePhraseLoader: Pet__FavoritePhraseLoader__output_type,
  },
  readonly parameters: Record<PropertyKey, never>,
};
