import type { Mutation____link__output_type } from './../__link/output_type';
import type { Mutation__SetTagline__parameters } from './parameters_type';

export type Mutation__SetTagline__param = {
  readonly data: {
    /**
     * Replace a pet's tagline.
     */
    readonly set_pet_tagline: {
      readonly pet: {
        /**
         * The pet's ID
         */
        readonly id: string;
        /**
         * If your pet was a superhero.
         */
        readonly tagline: string;
      };
    };
  };
  readonly parameters: Mutation__SetTagline__parameters;
  readonly link: Mutation____link__output_type;
};
