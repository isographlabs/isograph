import { type Pet____link__output_type } from '../../Pet/__link/output_type';

export type Query__smartestPet__param = {
  readonly data: {
    /**
All the pets! What more could you ask for?
    */
    readonly pets: ReadonlyArray<{
      /**
A store Link for the Pet type.
      */
      readonly __link: Pet____link__output_type,
      /**
Charisma? 100%
      */
      readonly stats: ({
        readonly intelligence: (number | null),
      } | null),
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
