import type { Mutation__SetTagline__parameters } from './parameters_type';

export type Mutation__SetTagline__param = {
  readonly data: {
    readonly set_pet_tagline: {
      readonly pet: {
        /**
If your pet was a superhero.
        */
        readonly tagline: string,
      },
    },
  },
  readonly parameters: Mutation__SetTagline__parameters,
};
