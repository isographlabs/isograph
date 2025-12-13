import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';

export type Query__PetNameList__param = {
  readonly data: {
    /**
All the pets! What more could you ask for?
    */
    readonly pets: ReadonlyArray<{
      /**
The pet's ID
      */
      readonly id: string,
      readonly fullName: Pet__fullName__output_type,
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
