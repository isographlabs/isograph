import type { Pet____link__output_type } from './../__link/output_type';
import type { Pet__PetCheckinsCard__parameters } from './parameters_type';

export type Pet__PetCheckinsCard__param = {
  readonly data: {
    /**
     * The pet's ID
     */
    readonly id: string;
    /**
     * Even pets use social media, mmkay.
     */
    readonly checkins: ReadonlyArray<{
      readonly id: string;
      readonly location: string;
      readonly time: string;
    }>;
  };
  readonly parameters: Pet__PetCheckinsCard__parameters;
  readonly link: Pet____link__output_type;
};
