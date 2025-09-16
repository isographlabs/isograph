import { type Pet__link__output_type } from '../../Pet/link/output_type';

export type Query__smartestPet__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      /**
A store Link for the Pet type.
      */
      readonly link: Pet__link__output_type,
      readonly stats: ({
        readonly intelligence: (number | null),
      } | null),
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
