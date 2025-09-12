import type { PetLink } from '../../Pet/link_type.ts';

export type Query__smartestPet__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      /**
A store Link for the Pet type.
      */
      readonly link: PetLink,
      readonly stats: ({
        readonly intelligence: (number | null),
      } | null),
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
